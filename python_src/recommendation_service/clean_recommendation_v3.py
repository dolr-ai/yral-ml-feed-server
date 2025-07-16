from config import Config
from concurrent.futures import ThreadPoolExecutor
from utils.bigquery_utils import BigQueryClient
from ast import literal_eval
import logging
import pandas as pd
import random
import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc
import random
from recommendation_service.consts import (
    VIDEO_EMBEDDINGS_TABLE,
    GLOBAL_POPULAR_VIDEOS_TABLE,
    VIDEO_INDEX_TABLE,
    REPORT_VIDEO_TABLE,
    VIDEO_NSFW_TABLE,
    VIDEO_UNIQUE_TABLE,
)
import json
import os
from datetime import datetime
import uuid

_LOGGER = logging.getLogger(__name__)

_LOGGER.setLevel(logging.INFO)


class CleanRecommendationV3:
    def __init__(self):
        cfg = Config()
        self.bq = BigQueryClient()
        # self.upstash_db = UpstashUtils()
        ### hyper-parameters
        self.sample_size = 10  # number of successful plays to sample
        self.video_bucket_name = cfg.get("video_bucket_name")
        self.logging = cfg.get("logging")
        self.log_dir = cfg.get("log_dir", "logs")

    def sample_successful_plays(self, successful_plays):
        """
        Samples a subset of successful plays based on their like status and watch duration.

        This function calculates a score for each video based on whether it was liked and the percentage
        of the video that was watched. It then performs weighted sampling to select a subset of videos.

        Args:
            successful_plays (list of dict): A list of dictionaries where each dictionary contains
                                             'video_uri', 'item_type' (either 'video_duration_watched' or 'like_video'),
                                             and 'percent_watched' (a float representing the percentage of the video watched).

        Returns:
            list of str: A list of video URIs that have been sampled based on their scores.
        """
        scores = {}
        for play in successful_plays:
            like_score = 1 if play["item_type"] == "like_video" else 0
            watch_duration_score = (
                play["percent_watched"] / 100
            )  # assuming the values are between 0 and 100 in the cannister
            score = (like_score + watch_duration_score) / 2
            scores[play["video_uri"]] = score

        sample_size = min(
            len(set([i["video_uri"] for i in successful_plays])), self.sample_size
        )
        uris = list(scores.keys())
        weights = list(scores.values())
        vdf_sample = random.choices(uris, weights=weights, k=sample_size)

        return vdf_sample

##
# Popularity
## 
    def get_popular_videos(
        self, watch_history_uris, num_results, user_principal_id
    ):  # TODO : add nsfw tag in the global popular videos l7d
        # Construct the SQL query
        video_ids = [
            uri.split("/")[-1].split(".")[0] for uri in watch_history_uris
        ]  # the script would break if the format is not .mp4
        watched_video_ids = ", ".join(
            f"'{video_id}'" for video_id in video_ids
        )  # will have to check till how much watch history is allowed in bigquery
        # reportee_user_id is the one who reports the video.
        if watched_video_ids != "":
            query = f"""
            SELECT video_id, global_popularity_score, nsfw_probability
            FROM {GLOBAL_POPULAR_VIDEOS_TABLE}
            WHERE video_id NOT IN ({watched_video_ids})
            AND is_nsfw = False AND nsfw_ec = 'neutral'
            AND NOT EXISTS (
                SELECT 1 FROM {REPORT_VIDEO_TABLE}
                WHERE SUBSTR(video_uri, 18, ABS(LENGTH(video_uri) - 21)) = video_id
                AND reportee_user_id  = '{user_principal_id}'
            )
            AND NOT EXISTS (
                SELECT 1 FROM yral_ds.video_deleted
                WHERE video_id = {GLOBAL_POPULAR_VIDEOS_TABLE}.video_id
            )
            AND EXISTS (
                SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                WHERE video_id = {GLOBAL_POPULAR_VIDEOS_TABLE}.video_id
            )
            AND nsfw_probability < 0.4
            ORDER BY global_popularity_score DESC
            LIMIT {int(4*num_results)}
            """  # TODO: Add nsfw tag
        else:
            query = f"""
            SELECT video_id, global_popularity_score, nsfw_probability
            FROM {GLOBAL_POPULAR_VIDEOS_TABLE}
            WHERE is_nsfw = False AND nsfw_ec = 'neutral'
            AND NOT EXISTS (
                SELECT 1 FROM {REPORT_VIDEO_TABLE}
                WHERE SUBSTR(video_uri, 18, ABS(LENGTH(video_uri) - 21)) = video_id
                AND reportee_user_id  = '{user_principal_id}'
            )
            AND NOT EXISTS (
                SELECT 1 FROM yral_ds.video_deleted
                WHERE video_id = {GLOBAL_POPULAR_VIDEOS_TABLE}.video_id
            )
            AND EXISTS (
                SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                WHERE video_id = {GLOBAL_POPULAR_VIDEOS_TABLE}.video_id
            )
            AND nsfw_probability < 0.4
            ORDER BY global_popularity_score DESC
            LIMIT {int(4*num_results)}
            """
        rdf = self.bq.query(query)  # rdf - recent - popular videos data frame
        video_ids = rdf["video_id"].tolist()

        if not len(video_ids):
            return []

        video_ids_string = ", ".join(f'"{video_id}"' for video_id in video_ids)

        fetch_post_ids = f"""with uri_mapping as 
(
    SELECT distinct
    uri,
    SUBSTR(uri, 18, ABS(LENGTH(uri) - 21)) AS video_id,
    (SELECT value FROM UNNEST(metadata) WHERE name = 'post_id') AS post_id,
    (SELECT value FROM UNNEST(metadata) WHERE name = 'timestamp') AS timestamp,
    (SELECT value FROM UNNEST(metadata) WHERE name = 'publisher_user_id') AS publisher_user_id
    from {VIDEO_EMBEDDINGS_TABLE} 
    WHERE NOT EXISTS (
        SELECT 1 FROM yral_ds.video_deleted
        WHERE gcs_video_id = uri
    )
)
select video_id, post_id, publisher_user_id 
from uri_mapping 
where video_id in ({video_ids_string})"""

        mdf = self.bq.query(fetch_post_ids)  # mdf - metadata dataframe
        mdf = mdf[(mdf.post_id.isna() == False) & (mdf.publisher_user_id.isna() == False)]
        return []  # muting popoularity for now
        return mdf["post_id publisher_user_id".split()].to_dict("records")

    def get_score_aware_recommendation(
        self, sample_uris, watch_history_uris, num_results=10, user_principal_id="test_principal"
    ):
        """
        Generates a list of recommended post IDs based on the sample URIs and watch history.

        Args:
            sample_uris (list of str): A list of URIs representing the sample videos.
            watch_history_uris (list of str): A list of URIs representing the user's watch history.
            num_results (int, optional): The number of results to return. Defaults to 10.
            user_principal_id (str, optional): The principal ID of the user. Defaults to "test_principal".

        Returns:
            list of dict: A list of dictionaries, where each dictionary contains 'publisher_user_id' and 'post_id'
                          keys corresponding to the recommended posts.
        """
        if len(sample_uris) == 0:
            return []

        watch_history_uris_string = ",".join([f"'{i}'" for i in watch_history_uris])
        sample_uris_string = ",".join([f"'{i}'" for i in sample_uris])
        search_breadth = 2 * ((int(num_results**0.5)) + 1)

        vs_query = f"""
        with search_result as (
        SELECT base.uri, base.post_id, base.publisher_user_id, distance FROM
        VECTOR_SEARCH(
            (
                SELECT * FROM {VIDEO_INDEX_TABLE}
                WHERE uri NOT IN ({watch_history_uris_string}) -- maintain a recency filter with date here / bloom
                AND is_nsfw = False AND nsfw_ec = 'neutral'
                AND post_id is not null 
                AND publisher_user_id is not null 
                AND NOT EXISTS (
                    SELECT 1 FROM {REPORT_VIDEO_TABLE}
                    WHERE video_uri = uri
                    AND reportee_user_id = '{user_principal_id}'
                )
                AND NOT EXISTS (
                    SELECT 1 FROM yral_ds.video_deleted
                    WHERE gcs_video_id = uri
                )
                AND EXISTS (
                    SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                    WHERE video_id = SUBSTR(uri, 18, ABS(LENGTH(uri) - 21))
                )
            ),
            'embedding',
            (
                SELECT embedding
                FROM {VIDEO_INDEX_TABLE}
                WHERE uri IN ({sample_uris_string})  
                AND is_nsfw = False AND nsfw_ec = 'neutral'
            ),
            top_k => 5000,
            options => '{{"fraction_lists_to_search":0.6}}' -- CAUTION: This is high at the moment owing to the sparsity of the data, as an when we will have good number of recent uploads, this has to go down!
        )
        )
        SELECT search_result.uri, search_result.post_id, search_result.publisher_user_id, search_result.distance, 
               video_nsfw_agg.probability as nsfw_probability,
               SUBSTR(search_result.uri, 18, ABS(LENGTH(search_result.uri) - 21)) AS video_id
        FROM search_result
        LEFT JOIN {VIDEO_NSFW_TABLE} as video_nsfw_agg
        ON search_result.uri = video_nsfw_agg.gcs_video_id
        where video_nsfw_agg.probability < 0.4
        ORDER BY distance
        LIMIT {4*num_results}
--        LIMIT {search_breadth}
        ;
        """  # TODO: ReIndexing with NSFW tag
        try:
            result_df = self.bq.query(vs_query).drop_duplicates(subset=["uri"])
        except Exception as e:
            _LOGGER.warning(f"Error in vector search query: {e}")
            return []
        return result_df.to_dict("records")

##
# Recency based exploration
## 

    def get_random_recent_recommendation(
        self, sample_uris, watch_history_uris, num_results=10, user_principal_id=None
    ):
        """
        Gets random recent recommendations, filtered by reported videos for the specific user.

        Args:
            sample_uris (list of str): A list of URIs representing the sample videos.
            watch_history_uris (list of str): A list of URIs representing the user's watch history.
            num_results (int, optional): The number of results to return. Defaults to 10.
            user_principal_id (str, optional): The principal ID of the user. Defaults to "test_principal".

        Returns:
            list of dict: A list of dictionaries with recommended videos.
        """
        if not len(watch_history_uris):
            query = f"""
            with recent_uploads as (
            SELECT uri, post_id, publisher_user_id, timestamp,
                   video_nsfw_agg.probability as nsfw_probability,
                   SUBSTR(uri, 18, ABS(LENGTH(uri) - 21)) AS video_id
            FROM {VIDEO_INDEX_TABLE} search_result
            LEFT JOIN {VIDEO_NSFW_TABLE} as video_nsfw_agg
            ON search_result.uri = video_nsfw_agg.gcs_video_id
            WHERE search_result.is_nsfw = False AND search_result.nsfw_ec = 'neutral'
            AND video_nsfw_agg.probability < 0.4
            AND NOT EXISTS (
                SELECT 1 FROM {REPORT_VIDEO_TABLE}
                WHERE video_uri = uri
                AND reportee_user_id = '{user_principal_id}'
            )
            AND NOT EXISTS (
                SELECT 1 FROM yral_ds.video_deleted
                WHERE gcs_video_id = uri
            )
            AND EXISTS (
                SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                WHERE video_id = SUBSTR(uri, 18, ABS(LENGTH(uri) - 21))
            )
            order by timestamp desc
            limit {4*num_results}
            )
            select * from recent_uploads
            order by RAND()
            limit {4*num_results}
            """

        else:
            watch_history_uris_string = ",".join([f"'{i}'" for i in watch_history_uris])
            query = f"""
            with recent_uploads as (
            SELECT uri, post_id, publisher_user_id, timestamp,
                   video_nsfw_agg.probability as nsfw_probability,
                   SUBSTR(uri, 18, ABS(LENGTH(uri) - 21)) AS video_id
            FROM {VIDEO_INDEX_TABLE} search_result
            LEFT JOIN {VIDEO_NSFW_TABLE} as video_nsfw_agg
            ON search_result.uri = video_nsfw_agg.gcs_video_id
            WHERE search_result.uri NOT IN ({watch_history_uris_string})
            AND search_result.is_nsfw = False AND search_result.nsfw_ec = 'neutral'
            AND video_nsfw_agg.probability < 0.4
            AND NOT EXISTS (
                SELECT 1 FROM {REPORT_VIDEO_TABLE}
                WHERE video_uri = uri
                AND reportee_user_id = '{user_principal_id}'
            )
            AND NOT EXISTS (
                SELECT 1 FROM yral_ds.video_deleted
                WHERE gcs_video_id = uri
            )
            AND EXISTS (
                SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                WHERE video_id = SUBSTR(uri, 18, ABS(LENGTH(uri) - 21))
            )
            order by timestamp desc
            limit {4*num_results}
            )
            select * from recent_uploads
            order by RAND()
            limit {4*num_results}
            """ # Use nsfw tag in this index

        result_df = self.bq.query(query).drop_duplicates(subset=["uri"])
        return result_df.to_dict("records")

##
# Recency based exploitation
## 

    def get_recency_aware_recommendation(
        self, sample_uris, watch_history_uris, num_results=10, user_principal_id="test_principal"
    ):
        """
        Generates a list of recommended post IDs based on the successful plays and watch history,
        applying weighted sampling based on video likes and watch duration. Maintains the search in the recently uploaded videos.

        Args:
            sample_uris (list of str): A list of URIs representing the sample videos.
                                             and 'percent_watched' (a float representing the percentage of the video watched).
            watch_history_uris (list of str): A list of URIs representing the user's watch history.
            num_results (int, optional): The number of results to return. Defaults to 10.
            user_principal_id (str, optional): The principal ID of the user. Defaults to "test_principal".

        Returns:
            list of dict: A list of dictionaries, where each dictionary contains 'publisher_user_id' and 'post_id'
                          keys corresponding to the recommended posts.
        """
        if not len(sample_uris):
            return []
        search_breadth = 2 * ((int(num_results**0.5)) + 1) # muting search breadth for the non duplicate high availability requirement
        watch_history_uris_string = ",".join([f"'{i}'" for i in watch_history_uris])
        sample_uris_string = ",".join([f"'{i}'" for i in sample_uris])
        # Not filtering sample URI for nsfw, but only filtering the result  -- moving forward, we should see less and less nsfw in sample URI and it should eventually diminish.
        vs_query = f"""
        with search_result as (
        SELECT base.uri, base.post_id, base.publisher_user_id, base.timestamp, distance FROM
        VECTOR_SEARCH(
            (
            SELECT * FROM {VIDEO_INDEX_TABLE} 
            WHERE uri NOT IN ({watch_history_uris_string})
            AND is_nsfw = False AND nsfw_ec = 'neutral'
            AND post_id is not null 
            AND publisher_user_id is not null 
            AND TIMESTAMP_TRUNC(TIMESTAMP(SUBSTR(timestamp, 1, 26)), MICROSECOND) > TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 2 DAY)
            AND NOT EXISTS (
                SELECT 1 FROM {REPORT_VIDEO_TABLE}
                WHERE video_uri = uri
                AND reportee_user_id = '{user_principal_id}'
            )
            AND NOT EXISTS (
                SELECT 1 FROM yral_ds.video_deleted
                WHERE gcs_video_id = uri
            )
            AND EXISTS (
                SELECT 1 FROM {VIDEO_UNIQUE_TABLE}
                WHERE video_id = SUBSTR(uri, 18, ABS(LENGTH(uri) - 21))
            )
            ),
            'embedding',
            (
            SELECT embedding
            FROM {VIDEO_INDEX_TABLE}
            WHERE uri IN ({sample_uris_string})
            AND is_nsfw = False AND nsfw_ec = 'neutral'
            AND post_id is not null
            AND publisher_user_id is not null
            ),
            top_k => 5000,
            options => '{{"fraction_lists_to_search":0.6}}' -- CAUTION: This is high at the moment owing to the sparsity of the data, as an when we will have good number of recent uploads, this has to go down!
            )
        )
        select search_result.uri, search_result.post_id, search_result.publisher_user_id, search_result.distance,
               video_nsfw_agg.probability as nsfw_probability,
               SUBSTR(search_result.uri, 18, ABS(LENGTH(search_result.uri) - 21)) AS video_id
        from search_result
        LEFT JOIN {VIDEO_NSFW_TABLE} as video_nsfw_agg
        ON search_result.uri = video_nsfw_agg.gcs_video_id
        where video_nsfw_agg.probability < 0.4
        ORDER BY distance 
        LIMIT {4*num_results}
--        LIMIT {search_breadth}
        """
        try:
            result_df = self.bq.query(vs_query).drop_duplicates(subset=["uri"])

        except Exception as e:
            _LOGGER.warning(f"Error in vector search query: {e}")
            return []
        return result_df.to_dict("records")

    def get_collated_recommendation(
        self, successful_plays, watch_history_uris, num_results=10, user_principal_id=None
    ):  # TODO: also get the feed type ()
        """
        Generates a list of recommended post IDs based on the successful plays and watch history,
        applying weighted sampling based on video likes and watch duration.

        Args:
            successful_plays (list of dict): List of dictionaries containing video play information.
            watch_history_uris (list of str): List of video URIs the user has watched.
            num_results (int, optional): Number of results to return. Defaults to 10.
            user_principal_id (str, optional): The principal ID of the user making the request. Defaults to "test_principal".

        Returns:
            MLFeedResponseV3: A response containing the recommended feed items.
        """
        sample_uris = (
            self.sample_successful_plays(successful_plays)
            if len(successful_plays) > 0
            else []
        )
        with ThreadPoolExecutor(max_workers=4) as executor:
            future_exploit = executor.submit(
                self.get_score_aware_recommendation,
                sample_uris,
                watch_history_uris,
                num_results,
                user_principal_id,
            )
            future_recency = executor.submit(
                self.get_recency_aware_recommendation,
                sample_uris,
                watch_history_uris,
                num_results,
                user_principal_id,
            )
            future_popular = executor.submit(
                self.get_popular_videos, 
                watch_history_uris, 
                num_results, 
                user_principal_id
            )
            future_random_recent = executor.submit(
                self.get_random_recent_recommendation,
                sample_uris,
                watch_history_uris,
                num_results,
                user_principal_id,
            )

        exploit_recommendation = future_exploit.result()
        recency_recommendation = future_recency.result()
        popular_recommendation = future_popular.result()
        random_recent_recommendation = future_random_recent.result()
        if self.logging:
            # Log the length of all feed types fetched
            log_timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            log_id = str(uuid.uuid4())
            
            _LOGGER.info("[" + log_timestamp + "] [ID: " + log_id + "] Feed lengths - "
                         "Exploit: " + str(len(exploit_recommendation)) + ", "
                         "Recency: " + str(len(recency_recommendation)) + ", "
                         "Popular: " + str(len(popular_recommendation)) + ", "
                         "Random Recent: " + str(len(random_recent_recommendation)))
            
            # Also print to stdout for immediate visibility
            print("Feed lengths - "
                  "Exploit: " + str(len(exploit_recommendation)) + ", "
                  "Recency: " + str(len(recency_recommendation)) + ", "
                  "Popular: " + str(len(popular_recommendation)) + ", "
                  "Random Recent: " + str(len(random_recent_recommendation)))

            url_template = "https://yral.com/hot-or-not/{publisher_user_id}/{post_id}"
            similar_videos = [url_template.format(publisher_user_id=item["publisher_user_id"], post_id=item["post_id"]) for item in exploit_recommendation]
            print(similar_videos)
            print("Similar videos: " + " ".join(similar_videos))

        def create_feed_response(feed_items):
            return video_recommendation_pb2.MLFeedResponseV3(
                feed=[
                    video_recommendation_pb2.MLPostItemResponseV3(
                        post_id=item["post_id"], 
                        publisher_user_id=item["publisher_user_id"],
                        nsfw_probability=item.get("nsfw_probability", 0.0),
                        video_id=item.get("video_id", "")
                    )
                    for item in feed_items
                ]
            )

        response_exploitation = [
            {
                "post_id": int(item["post_id"]), 
                "publisher_user_id": item["publisher_user_id"],
                "video_id": item.get("video_id", ""),
                "nsfw_probability": item.get("nsfw_probability", 0.0)
            }
            for item in exploit_recommendation
        ]
        response_exploration = [
            {
                "post_id": int(item["post_id"]), 
                "publisher_user_id": item["publisher_user_id"],
                "video_id": item.get("video_id", ""),
                "nsfw_probability": item.get("nsfw_probability", 0.0)
            }
            for item in popular_recommendation
        ]
        response_recency = [
            {
                "post_id": int(item["post_id"]), 
                "publisher_user_id": item["publisher_user_id"],
                "video_id": item.get("video_id", ""),
                "nsfw_probability": item.get("nsfw_probability", 0.0)
            }
            for item in recency_recommendation
        ]
        response_random_recent = [
            {
                "post_id": int(item["post_id"]), 
                "publisher_user_id": item["publisher_user_id"],
                "video_id": item.get("video_id", ""),
                "nsfw_probability": item.get("nsfw_probability", 0.0)
            }
            for item in random_recent_recommendation
        ]

        required_sample_size = self.sample_size
        current_sample_size = len(successful_plays)

        def calculate_exploit_score(
            len_sample, len_required
        ):  # to be replaced with RL based exploration exploitation
            if len_required == 0:
                return 0
            ratio = len_sample / len_required
            score = max(0, min(90, ratio * 90))
            return score

        exploit_score = calculate_exploit_score(
            current_sample_size, required_sample_size
        )
        exploration_score = 100 - exploit_score

        (
            exploitation_score,
            recency_exploitation_score,
            exploration_score,
            random_recent_score,
        ) = (
            exploit_score/2,
            exploit_score/2,
            exploration_score * (1/2),
            exploration_score * (1/2),
        )

        combined_feed = (
            response_exploitation
            + response_recency
            + response_exploration
            + response_random_recent
        )
        combined_weights = (
            [exploitation_score] * len(response_exploitation)
            + [recency_exploitation_score] * len(response_recency)
            + [exploration_score] * len(response_exploration)
            + [random_recent_score] * len(response_random_recent)
        )
        combined_feed_with_weights = list(zip(combined_feed, combined_weights))

        seen = {}
        for item, weight in combined_feed_with_weights:
            identifier = f"{item['post_id']}_{item['publisher_user_id']}"
            if identifier not in seen or seen[identifier][1] < weight:
                seen[identifier] = (item, weight)

        unique_combined_feed_with_weights = list(seen.values())
        combined_feed, combined_weights = zip(*unique_combined_feed_with_weights)

        sampled_feed = random.choices(
            combined_feed, weights=combined_weights, k=num_results
        )

        deduped_sampled_feed = []
        seen = set()
        for item in sampled_feed:
            identifier = f"{item['post_id']}_{item['publisher_user_id']}"
            if identifier not in seen:
                seen.add(identifier)
                deduped_sampled_feed.append(item)

        # for debugging
        if self.logging:
            _LOGGER.info(f"Length of returned feed: {len(deduped_sampled_feed)}")
        exploitation_count = sum(
            1 for item in deduped_sampled_feed if item in response_exploitation
        )
        recency_count = sum(1 for item in deduped_sampled_feed if item in response_recency)
        exploration_count = sum(
            1 for item in deduped_sampled_feed if item in response_exploration
        )
        random_recent_count = sum(
            1 for item in deduped_sampled_feed if item in response_random_recent
        )

        if self.logging:
            # Write all the data to a JSON file
            os.makedirs(self.log_dir, exist_ok=True)
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            log_file = os.path.join(self.log_dir, f"clean_recommendation_v3_{timestamp}_{user_principal_id}_{uuid.uuid4()}.json")
            
            log_data = {
                "timestamp": timestamp,
                "inputs": {
                    "successful_plays": successful_plays,
                    "watch_history_uris": watch_history_uris,
                    "num_results": num_results,
                    "user_principal_id": user_principal_id
                },
                "recommendations": {
                    "response_exploitation": response_exploitation,
                    "response_exploration": response_exploration,
                    "response_recency": response_recency,
                    "response_random_recent": response_random_recent,
                    "combined_feed": list(combined_feed),
                    "sampled_feed": deduped_sampled_feed
                },
                "stats": {
                    "exploitation_count": exploitation_count,
                    "recency_count": recency_count,
                    "exploration_count": exploration_count,
                    "random_recent_count": random_recent_count,
                    "exploitation_score": exploitation_score,
                    "exploration_score": exploration_score,
                    "recency_exploitation_score": recency_exploitation_score,
                    "random_recent_score": random_recent_score
                }
            }
            
            with open(log_file, 'w') as f:
                json.dump(log_data, f, indent=2, default=str)
            
            _LOGGER.info(f"Clean recommendation V3 data logged to {log_file}")
            
        response = create_feed_response(deduped_sampled_feed)
        return response


if __name__ == "__main__":

    _LOGGER.setLevel(logging.INFO)

    import sys
    handler = logging.StreamHandler(sys.stdout)
    formatter = logging.Formatter("[PID %(process)d] %(message)s")
    handler.setFormatter(formatter)
    _LOGGER.addHandler(handler)

    outer_videos_watched = [
    "gs://yral-videos/53ec98e3b7314d4a86038cc064047b2d.mp4"
    ] * 10  # outer prefix for variables that are also used in the logic code

    outer_successful_plays = outer_videos_watched[:10]
    # outer_successful_plays = ["gs://yral-videos/bb13dbff7ee3494bae5bcb7e9309c5fe.mp4"]*5
    outer_filter_responses = []

    # outer_videos_watched = []
    # outer_successful_plays = []
    outer_num_results = 25

    # input_request_parameters

    watch_history = [
        video_recommendation_pb2.WatchHistoryItemV3(video_id=i)
        for i in outer_videos_watched
    ]

    success_history = [
        video_recommendation_pb2.SuccessHistoryItemV3(
            video_id=i, item_type="like_video", percent_watched=random.random()
        )
        for i in outer_successful_plays
    ]

    filter_posts = [
        video_recommendation_pb2.MLPostItemV3(
            post_id=post_id, publisher_user_id=publisher_user_id, video_id=video_id
        )
        for post_id, publisher_user_id, video_id in outer_filter_responses
    ]

    # input requests
    request = video_recommendation_pb2.MLFeedRequestV3(
        # user_principal_id="test_principal_id_jay",
        user_principal_id="test_principal_random",
        watch_history=watch_history,
        success_history=success_history,
        filter_posts=filter_posts,
        num_results=outer_num_results,
    )

    # process_request
    successful_plays_ = [
        {
            "video_uri": item.video_id,
            "item_type": item.item_type,
            "percent_watched": item.percent_watched,
        }
        for item in request.success_history
    ]
    outer_watch_history_uris = [item.video_id for item in request.watch_history] + [
        item.video_id for item in request.filter_posts
    ]
    num_results = request.num_results
    user_principal_id = request.user_principal_id
    import time

    recommender = CleanRecommendationV3()
    start_time = time.time()
    feed = recommender.get_collated_recommendation(
        successful_plays_, outer_watch_history_uris, num_results, user_principal_id
    )
    end_time = time.time()

    _LOGGER.info(
        f"Time required to get the recommendation: {end_time - start_time:.2f} seconds"
    )
    # print(
    #     f"Time required to get the recommendation: {end_time - start_time:.2f} seconds"
    # )

    for item in feed.feed:
        publisher_user_id = item.publisher_user_id
        post_id = item.post_id
        nsfw_probability = item.nsfw_probability
        video_id = item.video_id
        url = f"https://yral.com/hot-or-not/{publisher_user_id}/{post_id}"
        print(f"URL: {url}")
        print(f"NSFW probability: {nsfw_probability}")
        print(f"Video ID: {video_id}")
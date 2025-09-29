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
    PROJECT_ID,
    DATASET,
    REPORT_VIDEO_TABLE,
)
from datetime import datetime
import ml_feed_reports_pb2
from storage_api_writer import BigQueryStorageWriteAppend

_LOGGER = logging.getLogger(__name__)

_LOGGER.setLevel(logging.INFO)


class ReportVideoV0:
    @staticmethod
    def neighbour_search_query(post_id: str, canister_id: str):
        return f"""
        SELECT canister_id, post_id, uri as video_uri
        FROM (
            SELECT base.uri, base.post_id, base.canister_id, distance 
            FROM VECTOR_SEARCH(
                (
                    SELECT * 
                    FROM `hot-or-not-feed-intelligence.yral_ds.video_index`
                    WHERE post_id IS NOT NULL 
                    AND canister_id IS NOT NULL 
                ),
                'embedding',
                (
                    SELECT embedding
                    FROM `hot-or-not-feed-intelligence.yral_ds.video_index`
                    WHERE post_id = '{post_id}' 
                    AND canister_id = '{canister_id}'
                ),
                top_k => 1000,
                options => '{{"fraction_lists_to_search":0.6}}' 
            ) 
            WHERE distance < 0.16
        )
        GROUP BY canister_id, post_id, uri
        """

    def __init__(self):
        self.bq = BigQueryClient()
        self.cfg = Config()

    def find_similar_videos(self, video_post_id, video_canister_id):
        """
        Find similar videos to the reported video using vector search

        Args:
            video_post_id (str): The post ID of the reported video
            video_canister_id (str): The canister ID of the reported video

        Returns:
            list: A list of similar videos with their canister IDs and post IDs
        """
        query = self.neighbour_search_query(
            post_id=video_post_id, canister_id=video_canister_id
        )
        try:
            result_df = self.bq.query(query)
            return result_df.to_dict("records")
        except Exception as e:
            _LOGGER.error(f"Error finding similar videos: {e}")
            return []

    def report_video_v0(
        self,
        reportee_user_id: str,
        reportee_canister_id: str,
        video_canister_id: str,
        video_post_id: str,
        video_id: str,
        reason: str,
    ):
        """
        Reports a video and stores the report in BigQuery

        Args:
            reportee_user_id (str): User ID of the person reporting the video
            reportee_canister_id (str): Canister ID of the person reporting the video
            video_canister_id (str): Canister ID of the reported video
            video_post_id (int/str): Post ID of the reported video
            video_id (str): ID of the reported video
            reason (str): Reason for reporting the video

        Returns:
            bool: True if the report was successfully stored, False otherwise
        """
        _LOGGER.info(
            f"Reporting video {video_id} from {video_canister_id} with post id {video_post_id} by {reportee_user_id}"
        )

        try:
            # Ensure video_post_id is an integer
            try:
                video_post_id_int = int(video_post_id)
            except (ValueError, TypeError):
                _LOGGER.error(
                    f"Invalid video_post_id: {video_post_id}, must be convertible to integer"
                )
                return False

            # Find similar videos
            # similar_videos = self.find_similar_videos(video_post_id, video_canister_id)
            similar_videos = []
            if similar_videos:
                _LOGGER.info(
                    f"Found {len(similar_videos)} similar videos to reported video"
                )

            # Prepare data for storage
            report_data = [
                {
                    "reportee_user_id": str(reportee_user_id),
                    "reportee_canister_id": str(reportee_canister_id),
                    "video_canister_id": str(video_canister_id),
                    "video_post_id": str(
                        video_post_id_int
                    ),  # Convert to string to match proto
                    "video_uri": str(
                        video_id
                    ),  # Renamed from video_id to video_uri to match proto
                    "parent_video_canister_id": "",  # Add empty parent fields
                    "parent_video_post_id": "",
                    "parent_video_uri": "",
                    "reason": str(reason),
                    "report_timestamp": str(datetime.now()),
                    "report_type": "reported_video",
                }
            ]

            # Add similar videos to report data with type "predicted_video"
            for similar_video in similar_videos:
                # Skip if it's the same as the reported video
                if (
                    str(similar_video["post_id"]) == str(video_post_id)
                    and similar_video["canister_id"] == video_canister_id
                ):
                    continue

                # Ensure post_id is an integer
                try:
                    similar_post_id = int(similar_video["post_id"])
                except (ValueError, TypeError):
                    _LOGGER.warning(
                        f"Skipping invalid post_id in similar video: {similar_video['post_id']}"
                    )
                    continue

                similar_video_report = {
                    "reportee_user_id": str(reportee_user_id),
                    "reportee_canister_id": str(reportee_canister_id),
                    "video_canister_id": str(similar_video["canister_id"]),
                    "video_post_id": str(
                        similar_post_id
                    ),  # Convert to string to match proto
                    "video_uri": str(
                        similar_video["video_uri"]
                    ),  # Using video_uri directly
                    "parent_video_canister_id": str(
                        video_canister_id
                    ),  # Set parent fields to the original video
                    "parent_video_post_id": str(video_post_id_int),
                    "parent_video_uri": str(video_id),
                    "reason": str(reason),
                    "report_timestamp": str(datetime.now()),
                    "report_type": "predicted_video",
                }
                report_data.append(similar_video_report)

            _LOGGER.info(
                f"Adding {len(report_data)} reports to BigQuery (1 reported + {len(report_data)-1} similar videos)"
            )

            # Write all reports to BigQuery in one batch
            table_id = REPORT_VIDEO_TABLE.split(".")[-1].replace("`", "")
            bigquery_storage_writer = BigQueryStorageWriteAppend(
                PROJECT_ID, DATASET, table_id
            )

            try:
                result = bigquery_storage_writer.append_rows_proto2(report_data)

                if result:
                    _LOGGER.info(
                        f"Successfully reported video {video_id} and {len(report_data)-1} similar videos"
                    )
                    return True
                else:
                    _LOGGER.error(f"Failed to report videos")
                    return False
            except Exception as e:
                _LOGGER.error(f"Error in BigQueryStorageWriteAppend: {e}")
                return False

        except Exception as e:
            _LOGGER.error(f"Error reporting video: {e}")
            return False


if __name__ == "__main__":
    # Configure logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(message)s",
        handlers=[logging.StreamHandler()],
    )

    # Example usage
    reporter = ReportVideoV0()
    success = reporter.report_video_v0(
        reportee_user_id="test_user_123",
        reportee_canister_id="test_canister_id_jay",
        video_canister_id="76qol-iiaaa-aaaak-qelkq-cai",
        video_post_id="3178",
        video_id="gs://yral-videos/dc3f5cc8b7f04257be1943f321025b63.mp4",
        reason="duplicates",
    )

    print(f"Report submitted successfully: {success}")

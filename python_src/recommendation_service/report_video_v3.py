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


class ReportVideoV3:
    @staticmethod
    def neighbour_search_query(video_id: str):
        uri = f"gs://yral-videos/{video_id}.mp4"
        return f"""
        SELECT publisher_user_id, post_id, uri as video_uri
        FROM (
            SELECT base.uri, base.post_id, base.publisher_user_id, distance 
            FROM VECTOR_SEARCH(
                (
                    SELECT * 
                    FROM `hot-or-not-feed-intelligence.yral_ds.video_index`
                    WHERE post_id IS NOT NULL 
                    AND publisher_user_id IS NOT NULL 
                ),
                'embedding',
                (
                    SELECT embedding
                    FROM `hot-or-not-feed-intelligence.yral_ds.video_index`
                    WHERE uri = '{uri}' 
                ),
                top_k => 1000,
                options => '{{"fraction_lists_to_search":0.6}}' 
            ) 
            WHERE distance < 0.16
        )
        GROUP BY publisher_user_id, post_id, uri
        """

    def __init__(self):
        self.bq = BigQueryClient()
        self.cfg = Config()

    def find_similar_videos(self, video_id: str):
        """
        Find similar videos to the reported video using vector search

        Args:
            video_id (str): The ID of the reported video

        Returns:
            list: A list of similar videos with their publisher_user_ids and post_ids
        """
        query = self.neighbour_search_query(
            video_id=video_id
        )
        try:
            result_df = self.bq.query(query)
            return result_df.to_dict("records")
        except Exception as e:
            _LOGGER.error(f"Error finding similar videos: {e}")
            return []

    def report_video_v3(
        self,
        reportee_user_id: str,
        video_id: str,
        reason: str,
    ):
        """
        Reports a video and stores the report in BigQuery

        Args:
            reportee_user_id (str): User ID of the person reporting the video
            video_id (str): ID of the reported video
            reason (str): Reason for reporting the video

        Returns:
            bool: True if the report was successfully stored, False otherwise
        """
        _LOGGER.info(
            f"Reporting video {video_id} by {reportee_user_id}"
        )

        try:
            # Find similar videos
            similar_videos = self.find_similar_videos(video_id)
            if similar_videos:
                _LOGGER.info(
                    f"Found {len(similar_videos)} similar videos to reported video"
                )

            # Prepare data for storage
            report_data = [
                {
                    "reportee_user_id": str(reportee_user_id),
                    "reportee_canister_id": "",
                    "video_canister_id": "",
                    "video_post_id": "",
                    "video_uri": str(
                        video_id
                    ), 
                    "parent_video_canister_id": "",
                    "parent_video_post_id": "",
                    "parent_video_uri": "",
                    "report_timestamp": str(datetime.now()),
                    "report_type": "reported_video",
                    "reason": str(reason),
                }
            ]

            # Add similar videos to report data with type "predicted_video"
            for similar_video in similar_videos:
                # Skip if it's the same as the reported video
                if (
                    str(similar_video["post_id"]) == str(video_id)
                    and similar_video["publisher_user_id"] == reportee_user_id
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
                    "reportee_canister_id": "",
                    "video_canister_id": "",
                    "video_post_id": "",
                    "video_uri": str(
                        similar_video["video_uri"]
                    ),  # Using video_uri directly
                    "parent_video_canister_id": "",
                    "parent_video_post_id": "",
                    "parent_video_uri": "",
                    "report_timestamp": str(datetime.now()),
                    "report_type": "predicted_video",
                    "reason": str(reason),
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
    reporter = ReportVideoV3()
    success = reporter.report_video_v3(
        reportee_user_id="test_user_jay_2",
        video_id="gs://yral-videos/dc3f5cc8b7f04257be1943f321025b63.mp4",
        reason="duplicates",
    )

    print(f"Report submitted successfully: {success}")

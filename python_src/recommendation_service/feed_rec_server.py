import os
from concurrent import futures
import contextlib
import datetime
import logging
import multiprocessing
import random
import socket
import sys
import time
from typing import Any
import jwt

# import consts

import grpc
from grpc_reflection.v1alpha import reflection
from recommendation_service import video_recommendation_pb2
from recommendation_service import video_recommendation_pb2_grpc

from report_video_v3 import ReportVideoV3
from utils.upstash_utils import UpstashUtils
from simple_recommendation_v0 import SimpleRecommendationV0
from clean_recommendation_v0 import CleanRecommendationV0
from nsfw_feed_recommendation_v0 import NsfwRecommendationV0
from clean_recommendation_report_filtered_v0 import CleanRecommendationReportFilteredV0
from nsfw_feed_recommendation_report_filtered_v0 import (
    NsfwRecommendationReportFilteredV0,
)
from report_video_v0 import ReportVideoV0
# Import v2 recommendation classes
from clean_recommendation_v2 import CleanRecommendationV2
from nsfw_recommendation_v2 import NsfwRecommendationV2
from combined_recommendation_v2 import CombinedRecommendationV2
# Import deduped v2 recommendation classes
from clean_recommendation_v2_deduped import CleanRecommendationV2Deduped
from nsfw_recommendation_v2_deduped import NsfwRecommendationV2Deduped
from combined_recommendation_v2_deduped import CombinedRecommendationV2Deduped

from clean_recommendation_v3 import CleanRecommendationV3
from nsfw_recommendation_v3 import NsfwRecommendationV3
from combined_recommendation_v3 import CombinedRecommendationV3

_LOGGER = logging.getLogger(__name__)

_ONE_DAY = datetime.timedelta(days=1)
_THREAD_CONCURRENCY = 10  # heuristic
_PROCESS_COUNT = multiprocessing.cpu_count()

# _PROCESS_COUNT = 1  # TODO: stage change book mark
# _THREAD_CONCURRENCY = 1  # heuristic TODO: stage change book mark
_BIND_ADDRESS = "[::]:50059"  # Fixed bind address


_AUTH_HEADER_KEY = "authorization"

# _PUBLIC_KEY = consts.RECSYS_JWT_PUB_KEY
# _JWT_PAYLOAD = {
#     "sub": "yral-recsys-server",
#     "company": "gobazzinga",
# }


# class SignatureValidationInterceptor(grpc.ServerInterceptor):
#     def __init__(self):
#         def abort(ignored_request, context):
#             context.abort(grpc.StatusCode.UNAUTHENTICATED, "Invalid signature")

#         self._abort_handler = grpc.unary_unary_rpc_method_handler(abort)

#     def intercept_service(self, continuation, handler_call_details):
#         metadata_dict = dict(handler_call_details.invocation_metadata)
#         token = metadata_dict[_AUTH_HEADER_KEY].split()[1]
#         payload = jwt.decode(
#             token,
#             _PUBLIC_KEY,
#             algorithms=["EdDSA"],
#         )

#         if payload == _JWT_PAYLOAD:
#             return continuation(handler_call_details)
#         else:
#             print(f"Received payload: {payload}")
#             return self._abort_handler


class MLFeedServicer(video_recommendation_pb2_grpc.MLFeedServicer):
    def __init__(self):
        self.recommender = SimpleRecommendationV0()
        self.clean_recommender = CleanRecommendationV0()
        self.nsfw_recommender = NsfwRecommendationV0()
        self.clean_recommender_report_filtered_v0 = (
            CleanRecommendationReportFilteredV0()
        )
        self.nsfw_recommender_report_filtered_v0 = NsfwRecommendationReportFilteredV0()
        self.report_handler = ReportVideoV0()
        self.report_handler_v3 = ReportVideoV3()
        # Initialize v2 recommenders
        self.clean_recommender_v2 = CleanRecommendationV2()
        self.nsfw_recommender_v2 = NsfwRecommendationV2()
        self.combined_recommender_v2 = CombinedRecommendationV2()
        # Initialize deduped v2 recommenders
        self.clean_recommender_v2_deduped = CleanRecommendationV2Deduped()
        self.nsfw_recommender_v2_deduped = NsfwRecommendationV2Deduped()
        self.combined_recommender_v2_deduped = CombinedRecommendationV2Deduped()

        # Initialize v3 recommenders
        self.clean_recommender_v3 = CleanRecommendationV3()
        self.nsfw_recommender_v3 = NsfwRecommendationV3()
        self.combined_recommender_v3 = CombinedRecommendationV3()
        return

    def get_ml_feed(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.recommender.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )

    def get_ml_feed_clean(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        # successful_plays_uris = [item.video_id for item in request.success_history]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.clean_recommender.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )

    def get_ml_feed_nsfw(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.nsfw_recommender.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )

    def get_ml_feed_clean_v0(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.clean_recommender.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_nsfw_v0(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.nsfw_recommender.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_clean_v1(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.clean_recommender_report_filtered_v0.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_nsfw_v1(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.nsfw_recommender_report_filtered_v0.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_clean_v2(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.clean_recommender_v2.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_nsfw_v2(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.nsfw_recommender_v2.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_combined(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.combined_recommender_v2.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_clean_v2_deduped(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.clean_recommender_v2_deduped.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_nsfw_v2_deduped(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.nsfw_recommender_v2_deduped.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def get_ml_feed_combined_deduped(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results
        user_canister_id = request.canister_id

        return self.combined_recommender_v2_deduped.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
            user_canister_id=user_canister_id,
        )

    def report_video(self, request, context):
        reportee_user_id = request.reportee_user_id
        reportee_canister_id = request.reportee_canister_id
        video_canister_id = request.video_canister_id
        video_post_id = request.video_post_id
        video_id = request.video_id
        reason = request.reason

        print(
            f"Reporting video {video_id} from {video_canister_id} with post id {video_post_id} by {reportee_user_id}"
        )

        # Use the report_video_v0 method to handle the report
        success = self.report_handler.report_video_v0(
            reportee_user_id=reportee_user_id,
            reportee_canister_id=reportee_canister_id,
            video_canister_id=video_canister_id,
            video_post_id=video_post_id,
            video_id=video_id,
            reason=reason,
        )

        return video_recommendation_pb2.VideoReportResponse(success=success)

    def report_video_v3(self, request, context):
        reportee_user_id = request.reportee_user_id
        video_id = request.video_id
        reason = request.reason

        success = self.report_handler_v3.report_video_v3(
            reportee_user_id=reportee_user_id,
            video_id=video_id,
            reason=reason,
        )
        
        return video_recommendation_pb2.VideoReportResponseV3(success=success)

    def get_ml_feed_clean_v3(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.clean_recommender_v3.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )
    
    def get_ml_feed_nsfw_v3(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id, 
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.nsfw_recommender_v3.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )
    
    def get_ml_feed_combined_v3(self, request, context):
        watch_history_uris = [item.video_id for item in request.watch_history] + [
            item.video_id for item in request.filter_posts
        ]
        watch_history_uris = [
            uri if uri.startswith("gs://") else f"gs://yral-videos/{uri}.mp4"
            for uri in watch_history_uris
        ]
        successful_plays = [
            {
                "video_uri": item.video_id,
                "item_type": item.item_type,
                "percent_watched": item.percent_watched,
            }
            for item in request.success_history
        ]
        num_results = request.num_results

        return self.combined_recommender_v3.get_collated_recommendation(
            successful_plays=successful_plays,
            watch_history_uris=watch_history_uris,
            num_results=num_results,
        )


def _wait_forever(server):
    try:
        while True:
            time.sleep(_ONE_DAY.total_seconds())
    except KeyboardInterrupt:
        print("Killing the server")
        server.stop(None)
        import os  # for testing env

        os.system("lsof -ti :50059 | xargs kill -9")


def _run_server():
    _LOGGER.info("Starting new server.")
    options = (("grpc.so_reuseport", 1),)

    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=_THREAD_CONCURRENCY),
        # interceptors=(SignatureValidationInterceptor(),),
        options=options,
    )
    video_recommendation_pb2_grpc.add_MLFeedServicer_to_server(MLFeedServicer(), server)
    SERVICE_NAMES = (
        video_recommendation_pb2.DESCRIPTOR.services_by_name["MLFeed"].full_name,
        reflection.SERVICE_NAME,
    )
    reflection.enable_server_reflection(SERVICE_NAMES, server)
    server.add_insecure_port(_BIND_ADDRESS)
    server.start()
    _LOGGER.info(f"Server started on {_BIND_ADDRESS}")
    _wait_forever(server)


def main():
    multiprocessing.set_start_method("spawn", force=True)
    _LOGGER.info(f"Binding to '{_BIND_ADDRESS}'")
    sys.stdout.flush()

    workers = []
    for _ in range(_PROCESS_COUNT):
        worker = multiprocessing.Process(target=_run_server)
        worker.start()
        workers.append(worker)
    for worker in workers:
        worker.join()


if __name__ == "__main__":
    handler = logging.StreamHandler(sys.stdout)
    formatter = logging.Formatter("[PID %(process)d] %(message)s")
    handler.setFormatter(formatter)
    _LOGGER.addHandler(handler)
    _LOGGER.setLevel(logging.INFO)
    main()
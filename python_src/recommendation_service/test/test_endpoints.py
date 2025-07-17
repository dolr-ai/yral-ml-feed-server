import random
import time

import grpc  # type: ignore
import video_recommendation_pb2 as pb  # type: ignore
import video_recommendation_pb2_grpc as pb_grpc  # type: ignore

from payload_results_to_debug import DEBUG_PAYLOAD, DEBUG_RESULT

video_ids = DEBUG_PAYLOAD
WATCH_HISTORY_VIDEO_IDS = [i['video_id'] for i in DEBUG_PAYLOAD['watch_history']]
SUCCESSFUL_PLAYS = [i['video_id'] for i in DEBUG_PAYLOAD['success_history']]
FILTER_POSTS_VIDEO_IDS = [i['video_id'] for i in DEBUG_PAYLOAD['filter_posts']]

FILTER_POSTS_V2 = [
    (1, "test_canister", filter_post_video_id)
    for filter_post_video_id in FILTER_POSTS_VIDEO_IDS
]

FILTER_POSTS_V3 = [
    # (post_id, publisher_user_id, video_id)
    (1, "publisher_user_1", filter_post_video_id)
    for filter_post_video_id in FILTER_POSTS_VIDEO_IDS
]

from typing import Any, cast, List 
pb = cast(Any, pb)
SERVER_ADDRESS = "localhost:50059"
NUM_RESULTS = 110


def _random_video_ids(n: int) -> List[str]:
    """Generate *n* random video-id-like strings for testing."""
    return [f"gs://yral-videos/{random.getrandbits(128):032x}.mp4" for _ in range(n)]


def _build_v2_request(num_results: int = NUM_RESULTS):  # type: ignore
    """Construct a MLFeedRequest for the v2 API family using hard-coded data."""
    watch_history = [pb.WatchHistoryItem(video_id=vid) for vid in WATCH_HISTORY_VIDEO_IDS]

    success_history = [
        pb.SuccessHistoryItem(
            video_id=vid, item_type="like_video", percent_watched=random.random()
        )
        for vid in SUCCESSFUL_PLAYS
    ]

    filter_posts = [
        pb.MLPostItem(post_id=pid, canister_id=can_id, video_id=vid)
        for pid, can_id, vid in FILTER_POSTS_V2
    ]

    return pb.MLFeedRequest(
        canister_id="test_canister",
        watch_history=watch_history,
        success_history=success_history,
        filter_posts=filter_posts,
        num_results=num_results,
    )


def _build_v3_request(num_results: int = NUM_RESULTS):  # type: ignore
    """Construct a MLFeedRequestV3 for the v3 API family using hard-coded data."""
    watch_history = [pb.WatchHistoryItemV3(video_id=vid) for vid in VIDEO_IDS[:6]]

    success_history = [
        pb.SuccessHistoryItemV3(
            video_id=vid, item_type="like_video", percent_watched=random.random()
        )
        for vid in SUCCESSFUL_PLAYS
    ]

    filter_posts = [
        pb.MLPostItemV3(post_id=pid, publisher_user_id=pub_id, video_id=vid)
        for pid, pub_id, vid in FILTER_POSTS_V3
    ]

    return pb.MLFeedRequestV3(
        user_principal_id="test_user",  # Fake principal ID for local testing.
        watch_history=watch_history,
        success_history=success_history,
        filter_posts=filter_posts,
        num_results=num_results,
    )


def _get_stub() -> pb_grpc.MLFeedStub:
    """Create a gRPC stub connected to the local server."""
    channel = grpc.insecure_channel(SERVER_ADDRESS)
    return pb_grpc.MLFeedStub(channel)


def _pretty_print_feed(feed, title: str) -> None:
    print("-" * 80)
    print(title)
    for item in feed:
        if hasattr(item, "publisher_user_id"):
            # V3 response
            print(
                f"https://yral.com/hot-or-not/{item.publisher_user_id}/{item.post_id} | "
                f"NSFW: {getattr(item, 'nsfw_probability', 'N/A'):.3f} | Video: {item.video_id}"
            )
        else:
            # V2 response
            print(
                f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id} | "
                f"NSFW: {getattr(item, 'nsfw_probability', 'N/A'):.3f} | Video: {getattr(item, 'video_id', 'N/A')}"
            )
    print("-" * 80)



def test_get_ml_feed_clean_v2(request=_build_v2_request()) -> None:
    """Call get_ml_feed_clean_v2 and assert that a non-empty feed is returned."""
    stub = _get_stub()
    response: pb.MLFeedResponseV2 = stub.get_ml_feed_clean_v2(request)
    _pretty_print_feed(response.feed, "CLEAN FEED V2")
    assert isinstance(response, pb.MLFeedResponseV2)
    return response


def test_get_ml_feed_nsfw_v2(request=_build_v2_request()) -> None:
    stub = _get_stub()
    response: pb.MLFeedResponseV2 = stub.get_ml_feed_nsfw_v2(request)
    _pretty_print_feed(response.feed, "NSFW FEED V2")
    assert isinstance(response, pb.MLFeedResponseV2)


def test_get_ml_feed_combined_v2(request=_build_v2_request(num_results=10)) -> None:
    stub = _get_stub()
    response: pb.MLFeedResponseV2 = stub.get_ml_feed_combined(request)
    _pretty_print_feed(response.feed, "COMBINED FEED V2")
    assert isinstance(response, pb.MLFeedResponseV2)


def test_get_ml_feed_clean_v3(request=_build_v3_request()) -> None:
    stub = _get_stub()
    response: pb.MLFeedResponseV3 = stub.get_ml_feed_clean_v3(request)
    _pretty_print_feed(response.feed, "CLEAN FEED V3")
    assert isinstance(response, pb.MLFeedResponseV3)


def test_get_ml_feed_nsfw_v3(request=_build_v3_request()) -> None:
    stub = _get_stub()
    response: pb.MLFeedResponseV3 = stub.get_ml_feed_nsfw_v3(request)
    _pretty_print_feed(response.feed, "NSFW FEED V3")
    assert isinstance(response, pb.MLFeedResponseV3)


def test_get_ml_feed_combined_v3(request=_build_v3_request(num_results=10)) -> None:
    stub = _get_stub()
    response: pb.MLFeedResponseV3 = stub.get_ml_feed_combined_v3(request)
    _pretty_print_feed(response.feed, "COMBINED FEED V3")
    assert isinstance(response, pb.MLFeedResponseV3)


def test_report_video_v3() -> None:
    """Send a dummy report_video_v3 call and assert the server returns success."""
    stub = _get_stub()
    request = pb.VideoReportRequestV3(
        reportee_user_id="test_user",
        video_id="gs://yral-videos/dummy.mp4",
        reason="inappropriate",
    )
    response: pb.VideoReportResponseV3 = stub.report_video_v3(request)
    print("Report video v3 success:", response.success)
    assert isinstance(response, pb.VideoReportResponseV3)



##
if __name__ == "__main__":
    start = time.time()
    print("Running local endpoint smoke tests…")
    request = _build_v2_request()

    response = test_get_ml_feed_clean_v2()
    print(response)
    # test_get_ml_feed_nsfw_v2()
    # test_get_ml_feed_combined_v2()
    video_id_out = [item.video_id for item in response.feed]
    print(set(WATCH_HISTORY_VIDEO_IDS).intersection(video_id_out))

    # test_get_ml_feed_clean_v3()
    # test_get_ml_feed_nsfw_v3()
    # test_get_ml_feed_combined_v3()

    # test_report_video_v3()

    print(f"Completed in {time.time() - start:.2f}s")

    # checking duplicacy in response

    video_id_out = [item.video_id for item in response.feed]
    print(set(WATCH_HISTORY_VIDEO_IDS).intersection(video_id_out))




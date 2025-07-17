import random
import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc

# Sample data for testing

high_duplicate_video = "gs://yral-videos/53ec98e3b7314d4a86038cc064047b2d.mp4"

videos_watched = [high_duplicate_video] * 10

successful_plays = videos_watched[:5]
filter_responses = []


def run(port=50059):
    # Assuming the server is running on localhost and port 50059
    with grpc.insecure_channel(f"localhost:{port}") as channel:
        stub = video_recommendation_pb2_grpc.MLFeedStub(channel)

        # Create a user canister ID for testing
        test_user_canister_id = "test_user_canister_123"
        
        request = video_recommendation_pb2.MLFeedRequest(
            canister_id=test_user_canister_id,  # This will be used to filter reports
            watch_history=[
                video_recommendation_pb2.WatchHistoryItem(video_id=i)
                for i in videos_watched
            ],
            success_history=[
                video_recommendation_pb2.SuccessHistoryItem(
                    video_id=i, item_type="like_video", percent_watched=random.random()
                )
                for i in successful_plays
            ],
            filter_posts=[
                video_recommendation_pb2.MLPostItem(
                    post_id=post_id, canister_id=canister_id, video_id=video_id
                )
                for post_id, canister_id, video_id in filter_responses
            ],
            num_results=25,
        )
        try:
            # Test the new report-filtered endpoints
            print(f"Testing with user canister ID: {test_user_canister_id}")
            response_nsfw_v1 = stub.get_ml_feed_nsfw_v1(request)
            response_clean_v1 = stub.get_ml_feed_clean_v1(request)

            print("NSFW FEED V1 (Report Filtered)")
            for item in response_nsfw_v1.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
            print("=" * 100)

            print("CLEAN FEED V1 (Report Filtered)")
            for item in response_clean_v1.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
            print("=" * 100)

        except grpc.RpcError as e:
            print(f"RPC failed: {e.code()} {e.details()}")


if __name__ == "__main__":
    import time

    start_time = time.time()
    run()
    end_time = time.time()
    print(f"Time required to run main: {end_time - start_time} seconds")

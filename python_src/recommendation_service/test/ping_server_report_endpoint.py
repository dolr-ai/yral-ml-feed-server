import random
import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc


def run(port=50059):
    # Assuming the server is running on localhost and port 50059
    with grpc.insecure_channel(f"localhost:{port}") as channel:
        stub = video_recommendation_pb2_grpc.MLFeedStub(channel)
        # Create a test request with dummy data
        # response = stub.get_ml_feed(video_recommendation_pb2.MLFeedRequest(canister_id="123"))
        # print("Client received: ", response.feed)
        # return

        request = video_recommendation_pb2.MLFeedRequest(
            canister_id="test_cannister",
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
            # response = stub.get_ml_feed(request)
            response = stub.get_ml_feed_nsfw(request)
            # print("Client received: ", response.feed)
            for item in response.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
        except grpc.RpcError as e:
            print(f"RPC failed: {e.code()} {e.details()}")


if __name__ == "__main__":
    import time

    start_time = time.time()
    run()
    end_time = time.time()
    print(f"Time required to run main: {end_time - start_time} seconds")

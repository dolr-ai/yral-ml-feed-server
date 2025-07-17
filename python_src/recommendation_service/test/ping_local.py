import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc


videos_watched = [
    "gs://yral-videos/cc75ebcdfcd04163bee6fcf37737f8bd.mp4",
    "gs://yral-videos/a8e1035908cc4e6e84f1792f8d655f25.mp4",
    "gs://yral-videos/9072dc569fe24c6d8ed7504d111cd71f.mp4",
    "gs://yral-videos/5054ef5791024da1977b446165aa9fb6.mp4",
    "gs://yral-videos/831fc4a20f974090aa7da3bedc9b0499.mp4",
    "gs://yral-videos/17e4f909dbf14a0b8b13e2b67ea5e54f.mp4",
    "gs://yral-videos/19d5ab6b30914e288db3f8b00dc5ab30.mp4",
    "gs://yral-videos/02b0c85d4da34c9aba1cf48b3b476ce8.mp4",
    "gs://yral-videos/53ec853631a844b48f57e893662daa2c.mp4",
    "gs://yral-videos/b3aac8dad2ef40b6bc987dcda57abd76.mp4",
]
successful_plays = videos_watched[:6]
filter_response = videos_watched[-1:]

# def run():
#     # Assuming the server is running on localhost:50059
#     with grpc.insecure_channel('localhost:50059') as channel:
#         stub = video_recommendation_pb2_grpc.MLFeedStub(channel)

#         # Create a MLFeedRequest with the provided data
#         request = video_recommendation_pb2.MLFeedRequest(
#             canister_id="test_canister",
#             watch_history=[
#                 video_recommendation_pb2.WatchHistoryItem(
#                     video_id=video_id
#                 ) for video_id in videos_watched
#             ],
#             success_history=[
#                 video_recommendation_pb2.SuccessHistoryItem(
#                     video_id=video_id
#                 ) for video_id in successful_plays
#             ],
#             filter_posts=[
#                 video_recommendation_pb2.MLPostItem(
#                     post_id=1,
#                     canister_id="asdf",
#                     video_id=video_id
#                 ) for video_id in filter_response
#             ],
#             num_results=5
#         )

#         # Send the request and receive the response
#         response = stub.get_ml_feed(request)
#         print("Client received: ", response)

# if __name__ == '__main__':
#     run()
import random
import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc


videos_watched = [
    "gs://yral-videos/cc75ebcdfcd04163bee6fcf37737f8bd.mp4",
    "gs://yral-videos/a8e1035908cc4e6e84f1792f8d655f25.mp4",
    "gs://yral-videos/9072dc569fe24c6d8ed7504d111cd71f.mp4",
    "gs://yral-videos/5054ef5791024da1977b446165aa9fb6.mp4",
    "gs://yral-videos/831fc4a20f974090aa7da3bedc9b0499.mp4",
    "gs://yral-videos/17e4f909dbf14a0b8b13e2b67ea5e54f.mp4",
    "gs://yral-videos/19d5ab6b30914e288db3f8b00dc5ab30.mp4",
    "gs://yral-videos/02b0c85d4da34c9aba1cf48b3b476ce8.mp4",
    "gs://yral-videos/53ec853631a844b48f57e893662daa2c.mp4",
    "gs://yral-videos/b3aac8dad2ef40b6bc987dcda57abd76.mp4",
]
successful_plays = videos_watched[:]
successful_plays = ["gs://yral-videos/bb13dbff7ee3494bae5bcb7e9309c5fe.mp4"] * 5
filter_responses = [
    (1, "test_canister", "gs://yral-videos/cc75ebcdfcd04163bee6fcf37737f8bd.mp4")
]

# videos_watched = successful_plays = filter_responses = []


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
            response = stub.get_ml_feed(request)
            response_nsfw = stub.get_ml_feed_nsfw_v2_deduped(request)
            response_clean = stub.get_ml_feed_clean_v2_deduped(request)
            response_combined = stub.get_ml_feed_combined_deduped(request)

            print("Client received: ", response.feed)
            print("NSFW FEED")
            for item in response_nsfw.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
                print(f"NSFW probability: {item.nsfw_probability}")
                print(f"Video ID: {item.video_id}")
            print("=" * 100)
            print("=" * 100)
            print("=" * 100)

            print("CLEAN FEED")
            for item in response_clean.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
                print(f"NSFW probability: {item.nsfw_probability}")
                print(f"Video ID: {item.video_id}")
            print("=" * 100)
            print("=" * 100)
            print("=" * 100)

            print("COMBINED FEED")
            for item in response_combined.feed:
                print(f"https://yral.com/hot-or-not/{item.canister_id}/{item.post_id}")
            
            print("=" * 100)
            print("=" * 100)
            print("=" * 100)
        
    
        except grpc.RpcError as e:
            print(f"RPC failed: {e.code()} {e.details()}")


if __name__ == "__main__":
    import time

    start_time = time.time()
    run()
    end_time = time.time()
    print(f"Time required to run main: {end_time - start_time} seconds")

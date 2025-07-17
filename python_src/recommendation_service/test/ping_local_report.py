import grpc
import video_recommendation_pb2
import video_recommendation_pb2_grpc
import random


def run(port=50059):
    # Assuming the server is running on localhost and port 50059
    with grpc.insecure_channel(f"localhost:{port}") as channel:
        stub = video_recommendation_pb2_grpc.MLFeedStub(channel)
        # Create a test request with dummy data
        # response = stub.get_ml_feed(video_recommendation_pb2.MLFeedRequest(canister_id="123"))
        # print("Client received: ", response.feed)
        # return

        request = video_recommendation_pb2.VideoReportRequest(
            reportee_user_id="test_user",
            reportee_canister_id="test_cannister",
            video_canister_id="f2ieq-tyaaa-aaaao-axgaa-cai",
            video_post_id=1,
            video_id="test_video",
            reason="test_reason",
        )
        try:
            # response = stub.get_ml_feed(request)
            response = stub.report_video(request)
            print(response)

        except grpc.RpcError as e:
            print(f"RPC failed: {e.code()} {e.details()}")


if __name__ == "__main__":
    import time

    start_time = time.time()
    run()
    end_time = time.time()
    print(f"Time required to run main: {end_time - start_time} seconds")

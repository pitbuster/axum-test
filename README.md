<div align="center">
  <h1>
    Axum Test Server
  </h1>

  [![crate](https://img.shields.io/crates/v/axum-test-server.svg)](https://crates.io/crates/axum-test-server)
  [![docs](https://docs.rs/axum-test-server/badge.svg)](https://docs.rs/axum-test-server)
</div>

This is for spinning up an Axum service, that you can then query directly.
This is primarily for testing Axum services.

```rust
  use ::axum::Router;
  use ::axum::routing::get;

  use ::axum_test_server::TestServer;

  async fn get_ping() -> &'static str {
      "pong!"
  }

  #[tokio::test]
  async fn it_sound_get() {
      // Build an application with a route.
      let app = Router::new()
          .route("/ping", get(get_ping))
          .into_make_service();

      // Run the server.
      let server = TestServer::new_with_random_address(app);

      // Get the request.
      let response = server
          .get("/ping")
          .await
          .unwrap();

      assert_eq!(response.contents, "pong!");
  }
```

One of the main benefits is you can spin up the server on a random port,
allowing you to run multiple servers in parallel.

use ::anyhow::Context;
use ::bytes::Bytes;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::http::header::AsHeaderName;
use ::http::header::HeaderName;
use ::http::header::SET_COOKIE;
use ::http::response::Parts;
use ::http::HeaderMap;
use ::http::HeaderValue;
use ::http::StatusCode;
use ::serde::de::DeserializeOwned;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Display;
use ::url::Url;

#[cfg(feature = "pretty-assertions")]
use ::pretty_assertions::{assert_eq, assert_ne};

use crate::internals::RequestPathFormatter;
use crate::internals::StatusCodeFormatter;

///
/// The `TestResponse` is the result of a request created using a [`TestServer`](crate::TestServer).
/// The `TestServer` builds a [`TestRequest`](crate::TestRequest), which when awaited,
/// will produce the response.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use ::axum::Json;
/// use ::axum::routing::Router;
/// use ::axum::routing::get;
/// use ::serde::Deserialize;
/// use ::serde::Serialize;
///
/// use ::axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
///
/// // This builds a `TestResponse`
/// let response = server.get(&"/todo").await;
/// #
/// # Ok(())
/// # }
/// ```
///
/// # Extracting Response
///
/// The functions [`TestResponse::json()`](crate::TestResponse::json()), [`TestResponse::text()`](crate::TestResponse::text()),
/// and [`TestResponse::form()`](crate::TestResponse::form()),
/// allow you to extract the underlying response content in different formats.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// # use ::axum::Json;
/// # use ::axum::routing::Router;
/// # use ::axum::routing::get;
/// # use ::serde::Deserialize;
/// # use ::serde::Serialize;
/// # use ::axum_test::TestServer;
/// #
/// # #[derive(Serialize, Deserialize, Debug)]
/// # struct Todo {}
/// #
/// # let app = Router::new()
/// #     .route(&"/test", get(|| async { "hello!" }));
/// #
/// # let server = TestServer::new(app)?;
/// let todo_response = server.get(&"/todo")
///         .await
///         .json::<Todo>();
///
/// let response_as_raw_text = server.get(&"/todo")
///         .await
///         .text();
/// #
/// # Ok(())
/// # }
/// ```
///
/// [`TestResponse::as_bytes()`](crate::TestResponse::as_bytes()) and [`TestResponse::into_bytes()`](crate::TestResponse::into_bytes()),
/// offer the underlying raw bytes to allow custom decoding.
///
/// Full code examples can be found within their documentation.
///
/// # Assertions
///
/// The result of a response can also be asserted using the many assertion functions.
///
/// ```rust
/// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
/// #
/// use ::axum::Json;
/// use ::axum::routing::Router;
/// use ::axum::routing::get;
/// use ::serde::Deserialize;
/// use ::serde::Serialize;
///
/// use ::axum_test::TestServer;
///
/// let app = Router::new()
///     .route(&"/test", get(|| async { "hello!" }));
///
/// let server = TestServer::new(app)?;
///
/// let response = server.get(&"/todo").await;
///
/// // These assertions will panic if they are not fulfilled by the response.
/// response.assert_status_ok();
/// response.assert_text("hello!");
/// #
/// # Ok(())
/// # }
/// ```
///
#[derive(Clone, Debug)]
pub struct TestResponse {
    request_format: RequestPathFormatter,

    /// This is the actual url that was used for the request.
    full_request_url: Url,
    headers: HeaderMap<HeaderValue>,
    status_code: StatusCode,
    response_body: Bytes,
}

impl TestResponse {
    pub(crate) fn new(
        request_format: RequestPathFormatter,
        full_request_url: Url,
        parts: Parts,
        response_body: Bytes,
    ) -> Self {
        Self {
            request_format,
            full_request_url,
            headers: parts.headers,
            status_code: parts.status,
            response_body,
        }
    }

    /// Returns the underlying response, extracted as a UTF-8 string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Json;
    /// use ::axum::routing::Router;
    /// use ::axum::routing::get;
    /// use ::serde_json::json;
    /// use ::serde_json::Value;
    ///
    /// use ::axum_test::TestServer;
    ///
    /// async fn route_get_todo() -> Json<Value> {
    ///     Json(json!({
    ///         "description": "buy milk",
    ///     }))
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a string on it's own.
    /// let raw_text = response.text();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.as_bytes()).to_string()
    }

    /// Deserializes the response, as Json, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Json;
    /// use ::axum::routing::Router;
    /// use ::axum::routing::get;
    /// use ::serde::Deserialize;
    /// use ::serde::Serialize;
    ///
    /// use ::axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Json<Todo> {
    ///     Json(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.json::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn json<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice::<T>(&self.as_bytes())
            .with_context(|| {
                let request_format = &self.request_format;

                format!("Deserializing response from Json, for request {request_format}")
            })
            .unwrap()
    }

    /// Deserializes the response, as Yaml, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::routing::Router;
    /// use ::axum::routing::get;
    /// use ::axum_yaml::Yaml;
    /// use ::serde::Deserialize;
    /// use ::serde::Serialize;
    ///
    /// use ::axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Yaml<Todo> {
    ///     Yaml(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.yaml::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "yaml")]
    #[must_use]
    pub fn yaml<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_yaml::from_slice::<T>(&self.as_bytes())
            .with_context(|| {
                let request_format = &self.request_format;

                format!("Deserializing response from YAML, for request {request_format}")
            })
            .unwrap()
    }

    /// Deserializes the response, as an urlencoded Form, into the type given.
    ///
    /// If deserialization fails then this will panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn test() -> Result<(), Box<dyn ::std::error::Error>> {
    /// #
    /// use ::axum::Form;
    /// use ::axum::routing::Router;
    /// use ::axum::routing::get;
    /// use ::serde::Deserialize;
    /// use ::serde::Serialize;
    ///
    /// use ::axum_test::TestServer;
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct Todo {
    ///     description: String,
    /// }
    ///
    /// async fn route_get_todo() -> Form<Todo> {
    ///     Form(Todo {
    ///         description: "buy milk".to_string(),
    ///     })
    /// }
    ///
    /// let app = Router::new()
    ///     .route(&"/todo", get(route_get_todo));
    ///
    /// let server = TestServer::new(app)?;
    /// let response = server.get(&"/todo").await;
    ///
    /// // Extract the response as a `Todo` item.
    /// let todo = response.form::<Todo>();
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn form<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        serde_urlencoded::from_bytes::<T>(&self.as_bytes())
            .with_context(|| {
                let request_format = &self.request_format;

                format!("Deserializing response from Form, for request {request_format}")
            })
            .unwrap()
    }

    /// Returns the raw underlying response as `Bytes`.
    #[must_use]
    pub fn as_bytes<'a>(&'a self) -> &'a Bytes {
        &self.response_body
    }

    /// Consumes this returning the underlying `Bytes`
    /// in the response.
    #[must_use]
    pub fn into_bytes<'a>(self) -> Bytes {
        self.response_body
    }

    /// The status_code of the response.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// The full URL that was used to produce this response.
    #[must_use]
    pub fn request_url(&self) -> Url {
        self.full_request_url.clone()
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first [`HeaderValue`](::http::HeaderValue) will be returned.
    ///
    /// `None` is returned when no header was found.
    #[must_use]
    pub fn maybe_header<N>(&self, header_name: N) -> Option<HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get(header_name).map(|h| h.to_owned())
    }

    /// Returns the headers returned from the response.
    #[must_use]
    pub fn headers<'a>(&'a self) -> &'a HeaderMap<HeaderValue> {
        &self.headers
    }

    /// Finds a header with the given name.
    /// If there are multiple headers with the same name,
    /// then only the first will be returned.
    ///
    /// If no header is found, then this will panic.
    #[must_use]
    pub fn header<N>(&self, header_name: N) -> HeaderValue
    where
        N: AsHeaderName + Display + Clone,
    {
        let debug_header = header_name.clone();
        self.headers
            .get(header_name)
            .map(|h| h.to_owned())
            .with_context(|| {
                let request_format = &self.request_format;

                format!("Cannot find header {debug_header}, for request {request_format}",)
            })
            .unwrap()
    }

    /// Iterates over all of the headers contained in the response.
    pub fn iter_headers<'a>(&'a self) -> impl Iterator<Item = (&'a HeaderName, &'a HeaderValue)> {
        self.headers.iter()
    }

    /// Iterates over all of the headers for a specific name, contained in the response.
    pub fn iter_headers_by_name<'a, N>(
        &'a self,
        header_name: N,
    ) -> impl Iterator<Item = &'a HeaderValue>
    where
        N: AsHeaderName,
    {
        self.headers.get_all(header_name).iter()
    }

    /// Finds a [`Cookie`] with the given name.
    /// If there are multiple matching cookies,
    /// then only the first will be returned.
    ///
    /// `None` is returned if no Cookie is found.
    #[must_use]
    pub fn maybe_cookie(&self, cookie_name: &str) -> Option<Cookie<'static>> {
        for cookie in self.iter_cookies() {
            if cookie.name() == cookie_name {
                return Some(cookie.into_owned());
            }
        }

        None
    }

    /// Finds a [`Cookie`](::cookie::Cookie) with the given name.
    /// If there are multiple matching cookies,
    /// then only the first will be returned.
    ///
    /// If no `Cookie` is found, then this will panic.
    #[must_use]
    pub fn cookie(&self, cookie_name: &str) -> Cookie<'static> {
        self.maybe_cookie(cookie_name)
            .with_context(|| {
                let request_format = &self.request_format;

                format!("Cannot find cookie {cookie_name}, for request {request_format}")
            })
            .unwrap()
    }

    /// Returns all of the cookies contained in the response,
    /// within a [`CookieJar`](::cookie::CookieJar) object.
    ///
    /// See the `cookie` crate for details.
    #[must_use]
    pub fn cookies(&self) -> CookieJar {
        let mut cookies = CookieJar::new();

        for cookie in self.iter_cookies() {
            cookies.add(cookie.into_owned());
        }

        cookies
    }

    /// Iterate over all of the cookies in the response.
    #[must_use]
    pub fn iter_cookies<'a>(&'a self) -> impl Iterator<Item = Cookie<'a>> {
        self.iter_headers_by_name(SET_COOKIE).map(|header| {
            let header_str = header
                .to_str()
                .with_context(|| {
                    let request_format = &self.request_format;

                    format!("Reading header 'Set-Cookie' as string, for request {request_format}",)
                })
                .unwrap();

            Cookie::parse(header_str)
                .with_context(|| {
                    let request_format = &self.request_format;

                    format!("Parsing 'Set-Cookie' header, for request {request_format}",)
                })
                .unwrap()
        })
    }

    /// This performs an assertion comparing the whole body of the response,
    /// against the text provided.
    #[track_caller]
    pub fn assert_text<C>(&self, other: C)
    where
        C: AsRef<str>,
    {
        let other_contents = other.as_ref();
        assert_eq!(other_contents, &self.text());
    }

    /// Deserializes the contents of the request as Json,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Json,
    /// then this will panic.
    #[track_caller]
    pub fn assert_json<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.json::<T>());
    }

    /// Deserializes the contents of the request as Yaml,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response is not Yaml,
    /// then this will panic.
    #[cfg(feature = "yaml")]
    #[track_caller]
    pub fn assert_yaml<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.yaml::<T>());
    }

    /// Deserializes the contents of the request as an url encoded form,
    /// and asserts it matches the value given.
    ///
    /// If `other` does not match, or the response cannot be deserialized,
    /// then this will panic.
    #[track_caller]
    pub fn assert_form<T>(&self, other: &T)
    where
        T: DeserializeOwned + PartialEq<T> + Debug,
    {
        assert_eq!(*other, self.form::<T>());
    }

    /// Assert that the status code is **within** the 2xx range.
    /// i.e. The range from 200-299.
    #[track_caller]
    pub fn assert_status_success(&self) {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let request_format = &self.request_format;

        assert!(
            200 <= status_code && status_code <= 299,
            "Expect status code within 2xx range, got {received_debug}, for request {request_format}"
        );
    }

    /// Assert that the status code is **outside** the 2xx range.
    /// i.e. A status code less than 200, or 300 or more.
    #[track_caller]
    pub fn assert_status_failure(&self) {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let request_format = &self.request_format;

        assert!(
            status_code < 200 || 299 < status_code,
            "Expect status code outside 2xx range, got {received_debug}, for request {request_format}",
        );
    }

    /// Assert the response status code is 400.
    #[track_caller]
    pub fn assert_status_bad_request(&self) {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    /// Assert the response status code is 404.
    #[track_caller]
    pub fn assert_status_not_found(&self) {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    /// Assert the response status code is 401.
    #[track_caller]
    pub fn assert_status_unauthorized(&self) {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }

    /// Assert the response status code is 403.
    #[track_caller]
    pub fn assert_status_forbidden(&self) {
        self.assert_status(StatusCode::FORBIDDEN)
    }

    /// Assert the response status code is 200.
    #[track_caller]
    pub fn assert_status_ok(&self) {
        self.assert_status(StatusCode::OK)
    }

    /// Assert the response status code is **not** 200.
    #[track_caller]
    pub fn assert_status_not_ok(&self) {
        self.assert_not_status(StatusCode::OK)
    }

    /// Assert the response status code matches the one given.
    #[track_caller]
    pub fn assert_status(&self, expected_status_code: StatusCode) {
        let status_code = self.status_code.as_u16();
        let received_debug = StatusCodeFormatter(self.status_code);
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let request_format = &self.request_format;

        assert_eq!(
            expected_status_code, status_code,
            "Expected status code {expected_debug}, got {received_debug}, for request {request_format}",
        );
    }

    /// Assert the response status code does **not** match the one given.
    #[track_caller]
    pub fn assert_not_status(&self, expected_status_code: StatusCode) {
        let expected_debug = StatusCodeFormatter(expected_status_code);
        let request_format = &self.request_format;

        assert_ne!(
            expected_status_code,
            self.status_code(),
            "Expected status code to not be {expected_debug}, it is, for request {request_format}",
        );
    }
}

impl From<TestResponse> for Bytes {
    fn from(response: TestResponse) -> Self {
        response.into_bytes()
    }
}

#[cfg(test)]
mod test_assert_success {
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::http::StatusCode;

    use crate::TestServer;

    pub async fn route_get_pass() -> StatusCode {
        StatusCode::OK
    }

    pub async fn route_get_fail() -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }

    #[tokio::test]
    async fn it_should_pass_when_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();

        let response = server.get(&"/pass").await;

        response.assert_status_success()
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_not_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();

        let response = server.get(&"/fail").expect_failure().await;

        response.assert_status_success()
    }
}

#[cfg(test)]
mod test_assert_failure {
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::http::StatusCode;

    use crate::TestServer;

    pub async fn route_get_pass() -> StatusCode {
        StatusCode::OK
    }

    pub async fn route_get_fail() -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }

    #[tokio::test]
    async fn it_should_pass_when_not_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();
        let response = server.get(&"/fail").expect_failure().await;

        response.assert_status_failure()
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_200() {
        let router = Router::new()
            .route(&"/pass", get(route_get_pass))
            .route(&"/fail", get(route_get_fail));

        let server = TestServer::new(router).unwrap();
        let response = server.get(&"/pass").await;

        response.assert_status_failure()
    }
}

#[cfg(test)]
mod test_assert_status {
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::http::StatusCode;

    use crate::TestServer;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_given_right_status_code() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_when_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_status(StatusCode::ACCEPTED);
    }
}

#[cfg(test)]
mod test_assert_not_status {
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::http::StatusCode;

    use crate::TestServer;

    pub async fn route_get_ok() -> StatusCode {
        StatusCode::OK
    }

    #[tokio::test]
    async fn it_should_pass_if_status_code_does_not_match() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server
            .get(&"/ok")
            .await
            .assert_not_status(StatusCode::ACCEPTED);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_status_code_matches() {
        let router = Router::new().route(&"/ok", get(route_get_ok));
        let server = TestServer::new(router).unwrap();

        server.get(&"/ok").await.assert_not_status(StatusCode::OK);
    }
}

#[cfg(test)]
mod test_into_bytes {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Json;
    use ::serde_json::json;
    use ::serde_json::Value;

    async fn route_get_json() -> Json<Value> {
        Json(json!({
            "message": "it works?"
        }))
    }

    #[tokio::test]
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        let bytes = server.get(&"/json").await.into_bytes();
        let text = String::from_utf8_lossy(&bytes);

        assert_eq!(text, r#"{"message":"it works?"}"#);
    }
}

#[cfg(test)]
mod test_json {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Json;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_deserialize_into_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/json").await.json::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum_yaml::Yaml;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_yaml() -> Yaml<ExampleResponse> {
        Yaml(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_deserialize_into_yaml() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/yaml").await.yaml::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(test)]
mod test_form {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Form;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_deserialize_into_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/form").await.form::<ExampleResponse>();

        assert_eq!(
            response,
            ExampleResponse {
                name: "Joe".to_string(),
                age: 20,
            }
        );
    }
}

#[cfg(test)]
mod test_assert_json {
    use crate::TestServer;

    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Form;
    use ::axum::Json;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_json_returned() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_json(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_json(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_json(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_assert_yaml {
    use crate::TestServer;

    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Form;
    use ::axum_yaml::Yaml;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_yaml() -> Yaml<ExampleResponse> {
        Yaml(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_yaml_returned() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app).unwrap();

        server.get(&"/yaml").await.assert_yaml(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/yaml", get(route_get_yaml));

        let server = TestServer::new(app).unwrap();

        server.get(&"/yaml").await.assert_yaml(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_form() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_yaml(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(test)]
mod test_assert_form {
    use crate::TestServer;

    use ::axum::routing::get;
    use ::axum::routing::Router;
    use ::axum::Form;
    use ::axum::Json;
    use ::serde::Deserialize;
    use ::serde::Serialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ExampleResponse {
        name: String,
        age: u32,
    }

    async fn route_get_form() -> Form<ExampleResponse> {
        Form(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    async fn route_get_json() -> Json<ExampleResponse> {
        Json(ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        })
    }

    #[tokio::test]
    async fn it_should_match_form_returned() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_form(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_different() {
        let app = Router::new().route(&"/form", get(route_get_form));

        let server = TestServer::new(app).unwrap();

        server.get(&"/form").await.assert_form(&ExampleResponse {
            name: "Julia".to_string(),
            age: 25,
        });
    }

    #[tokio::test]
    #[should_panic]
    async fn it_should_panic_if_response_is_json() {
        let app = Router::new().route(&"/json", get(route_get_json));

        let server = TestServer::new(app).unwrap();

        server.get(&"/json").await.assert_form(&ExampleResponse {
            name: "Joe".to_string(),
            age: 20,
        });
    }
}

#[cfg(test)]
mod test_text {
    use crate::TestServer;
    use ::axum::routing::get;
    use ::axum::routing::Router;

    #[tokio::test]
    async fn it_should_deserialize_into_text() {
        async fn route_get_text() -> String {
            "hello!".to_string()
        }

        let app = Router::new().route(&"/text", get(route_get_text));

        let server = TestServer::new(app).unwrap();

        let response = server.get(&"/text").await.text();

        assert_eq!(response, "hello!");
    }
}

from collections.abc import (
    AsyncIterable,
    AsyncIterator,
    Awaitable,
    Generator,
    Iterator,
    Mapping,
    Sequence,
)
from types import TracebackType
from typing import Any, Self, final

type _Primitive = str | int | float | bool | None
type QueryParams = (
    Mapping[str, _Primitive | Sequence[_Primitive]] | Sequence[tuple[str, _Primitive]]
)
type HeaderTypes = Headers | Mapping[str, str] | Sequence[tuple[str, str]]
type Body = bytes | str | Json | Form | AsyncIterable[bytes]

@final
class Json:
    """
    A wrapper class for JSON data to distinguish it from other types of data.
    """
    def __init__(self, value: Any, /) -> None: ...

@final
class Form:
    """
    A wrapper class for form data to distinguish it from other types of data.
    """
    def __init__(self, fields: QueryParams, /) -> None: ...

@final
class Headers(Mapping[str, str]):
    """
    Headers for HTTP requests and responses.
    """
    def __init__(self, headers: HeaderTypes | None = None, /) -> None: ...
    def __getitem__(self, key: str) -> str: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def __contains__(self, key: object) -> bool: ...
    def get_list(self, key: str) -> list[str]:
        """
        Get all values for a given header key as a list.

        :param key: The header key to look up.
        :type key: str
        :return: A list of header values for the given key.
        :rtype: list[str]
        """
    def multi_items(self) -> list[tuple[str, str]]:
        """
        Get all header key-value pairs as a list of tuples.

        :return: A list of tuples containing header key-value pairs.
        :rtype: list[tuple[str, str]]
        """

@final
class Request:
    """
    An HTTP request object
    """
    def __init__(
        self,
        method: str,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> None:
        """
        :param method: The HTTP method (e.g., 'GET', 'POST', etc.)
        :type method: str
        :param url: The URL for the request.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param body: Optional request body (for methods like POST or PUT).
        :type body: Body | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    @property
    def method(self) -> str:
        """
        The HTTP method of the request.
        """
    @property
    def url(self) -> str:
        """
        The URL of the request.
        """
    @property
    def headers(self) -> Headers:
        """
        The headers of the request.
        """

@final
class ResponseBody:
    @property
    def consumed(self) -> bool:
        """
        Whether the response body has been consumed or not.
        """
    @property
    def closed(self) -> bool:
        """
        Whether the response body has been closed or not.
        """
    def __aiter__(self) -> AsyncIterator[bytes]: ...
    def text_chunks(self) -> AsyncIterator[str]:
        """
        Get the response body as an asynchronous iterator of text chunks.

        :return: An asynchronous iterator yielding text chunks.
        :rtype: AsyncIterator[str]
        """
    def lines(self) -> AsyncIterator[str]:
        """
        Get the response body as an asynchronous iterator of lines.

        :return: An asynchronous iterator yielding lines of text.
        :rtype: AsyncIterator[str]
        """
    async def read(self) -> bytes:
        """
        Read the entire response body as bytes.

        :return: The response body as bytes.
        :rtype: bytes
        """
    async def text(self) -> str:
        """
        Read the entire response body as text.

        :return: The response body as a string.
        :rtype: str
        """
    async def json(self) -> Any:
        """
        Read the entire response body as JSON.

        :return: The response body parsed as JSON.
        :rtype: Any
        """

@final
class Response:
    """
    An HTTP response object.
    """
    @property
    def status(self) -> int:
        """
        The HTTP status code of the response.
        """
    @property
    def reason(self) -> str:
        """
        The reason phrase associated with the HTTP status code.
        """
    @property
    def is_success(self) -> bool:
        """
        Whether the response indicates a successful status (2xx).
        """
    @property
    def url(self) -> str:
        """
        The URL of the response.
        """
    @property
    def headers(self) -> Headers:
        """
        The headers of the response.
        """
    @property
    def request(self) -> Request:
        """
        The original request that resulted in this response.
        """
    @property
    def body(self) -> ResponseBody:
        """
        The body of the response.
        """
    def raise_for_status(self) -> Self:
        """
        Raise an exception if the response status indicates an error (4xx or 5xx).
        """
    async def close(self) -> None:
        """
        Close the response and release any associated resources.
        """
    async def __aenter__(self) -> Self: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...

@final
class PendingResponse(Awaitable[Response]):
    """
    An awaitable object representing a pending HTTP response.
    It can be awaited to obtain the final Response object once the request is completed.
    """
    def __await__(self) -> Generator[Any, None, Response]: ...
    async def __aenter__(self) -> Response: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...

@final
class Client:
    """
    An HTTP client for sending requests and receiving responses.
    """
    def __init__(
        self,
        *,
        base_url: str | None = None,
        headers: HeaderTypes | None = None,
        params: QueryParams | None = None,
        timeout: float | None = None,
    ) -> None:
        """
        :param base_url: Optional base URL for all requests made by this client.
        :type base_url: str | None
        :param headers: Optional default headers to include in all requests.
        :type headers: HeaderTypes | None
        :param params: Optional default query parameters to include in all requests.
        :type params: QueryParams | None
        :param timeout: Optional default timeout for all requests in seconds.
        :type timeout: float | None
        """
    def send(self, request: Request, /) -> PendingResponse:
        """
        Send an HTTP request and return a pending response.

        :param request: The HTTP request to send.
        :type request: Request
        :return: A pending response that can be awaited to obtain the final Response.
        :rtype: PendingResponse
        """
    def get(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a GET request to the specified URL.

        :param url: The URL to send the GET request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def head(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a HEAD request to the specified URL.

        :param url: The URL to send the HEAD request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def options(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send an OPTIONS request to the specified URL.
        
        :param url: The URL to send the OPTIONS request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def delete(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a DELETE request to the specified URL.
        
        :param url: The URL to send the DELETE request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param body: Optional body to include in the request.
        :type body: Body | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def post(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a POST request to the specified URL.
        
        :param url: The URL to send the POST request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param body: Optional body to include in the request.
        :type body: Body | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def put(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a PUT request to the specified URL.
        
        :param url: The URL to send the PUT request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param body: Optional body to include in the request.
        :type body: Body | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """
    def patch(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse:
        """
        Send a PATCH request to the specified URL.
        
        :param url: The URL to send the PATCH request to.
        :type url: str
        :param params: Optional query parameters to include in the URL.
        :type params: QueryParams | None
        :param headers: Optional headers to include in the request.
        :type headers: HeaderTypes | None
        :param body: Optional body to include in the request.
        :type body: Body | None
        :param timeout: Optional timeout for the request in seconds.
        :type timeout: float | None
        """

class HTTPError(Exception):
    """
    A base class for HTTP-related exceptions.
    """

class RequestError(HTTPError):
    """
    An exception raised for errors that occur during the request phase of an HTTP operation.
    """
    @property
    def request(self) -> Request:
        """
        The associated Request object that caused the error.
        """

class TransportError(RequestError):
    """
    An exception raised for errors that occur during the transport phase of an HTTP operation.
    """

class TimeoutException(TransportError):
    """
    An exception raised when a request times out.
    """

class RequestDenied(RequestError):
    """
    An exception raised when a request is denied due to policy or other reasons.
    """
    @property
    def policy(self) -> str:
        """
        The policy that caused the request to be denied.
        """
    @property
    def reason(self) -> str:
        """
        The reason why the request was denied.
        """

class TooManyRedirects(RequestError):
    """
    An exception raised when the maximum number of redirects is exceeded during an HTTP operation.
    """

class InvalidRequest(RequestError):
    """
    An exception raised for invalid HTTP requests.
    """

class ResponseError(HTTPError):
    """
    A base class for exceptions that occur during the response phase of an HTTP operation.
    """

    @property
    def response(self) -> Response:
        """
        The associated Response object that caused the error.
        """

class HTTPStatusError(ResponseError):
    """
    An exception raised for HTTP responses with error status codes (4xx or 5xx).
    """

class DecodingError(ResponseError):
    """
    An exception raised when there is an error decoding the response body.
    """

class ResponseTooLarge(ResponseError):
    """
    An exception raised when the response body exceeds the allowed size limit.
    """

    @property
    def limit(self) -> int:
        """
        The maximum allowed size limit for the response body.
        """

class BodyError(RuntimeError):
    """
    A base class for exceptions related to the response body.
    """

class BodyConsumed(BodyError):
    """
    An exception raised when an attempt is made to read a response body that has already been consumed.
    """

class BodyClosed(BodyError):
    """
    An exception raised when an attempt is made to read a response body that has already been closed.
    """

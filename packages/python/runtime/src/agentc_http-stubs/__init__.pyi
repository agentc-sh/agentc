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
    def __init__(self, value: Any, /) -> None: ...

@final
class Form:
    def __init__(self, fields: QueryParams, /) -> None: ...

@final
class Headers(Mapping[str, str]):
    def __init__(self, headers: HeaderTypes | None = None, /) -> None: ...
    def __getitem__(self, key: str) -> str: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def __contains__(self, key: object) -> bool: ...
    def get_list(self, key: str) -> list[str]: ...
    def multi_items(self) -> list[tuple[str, str]]: ...

@final
class Request:
    def __init__(
        self,
        method: str,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> None: ...
    @property
    def method(self) -> str: ...
    @property
    def url(self) -> str: ...
    @property
    def headers(self) -> Headers: ...

@final
class ResponseBody:
    @property
    def consumed(self) -> bool: ...
    @property
    def closed(self) -> bool: ...
    def __aiter__(self) -> AsyncIterator[bytes]: ...
    def text_chunks(self) -> AsyncIterator[str]: ...
    def lines(self) -> AsyncIterator[str]: ...
    async def read(self) -> bytes: ...
    async def text(self) -> str: ...
    async def json(self) -> Any: ...

@final
class Response:
    @property
    def status(self) -> int: ...
    @property
    def reason(self) -> str: ...
    @property
    def is_success(self) -> bool: ...
    @property
    def url(self) -> str: ...
    @property
    def headers(self) -> Headers: ...
    @property
    def request(self) -> Request: ...
    @property
    def body(self) -> ResponseBody: ...
    def raise_for_status(self) -> Self: ...
    async def close(self) -> None: ...
    async def __aenter__(self) -> Self: ...
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...

@final
class PendingResponse(Awaitable[Response]):
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
    def __init__(
        self,
        *,
        base_url: str | None = None,
        headers: HeaderTypes | None = None,
        params: QueryParams | None = None,
        timeout: float | None = None,
    ) -> None: ...
    def send(self, request: Request, /) -> PendingResponse: ...
    def get(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def head(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def options(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def delete(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def post(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def put(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...
    def patch(
        self,
        url: str,
        *,
        params: QueryParams | None = None,
        headers: HeaderTypes | None = None,
        body: Body | None = None,
        timeout: float | None = None,
    ) -> PendingResponse: ...

class HTTPError(Exception): ...

class RequestError(HTTPError):
    @property
    def request(self) -> Request: ...

class TransportError(RequestError): ...
class TimeoutException(TransportError): ...

class RequestDenied(RequestError):
    @property
    def policy(self) -> str: ...
    @property
    def reason(self) -> str: ...

class TooManyRedirects(RequestError): ...
class InvalidRequest(RequestError): ...

class ResponseError(HTTPError):
    @property
    def response(self) -> Response: ...

class HTTPStatusError(ResponseError): ...
class DecodingError(ResponseError): ...

class ResponseTooLarge(ResponseError):
    @property
    def limit(self) -> int: ...

class BodyError(RuntimeError): ...
class BodyConsumed(BodyError): ...
class BodyClosed(BodyError): ...

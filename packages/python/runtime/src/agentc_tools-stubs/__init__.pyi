from collections.abc import Callable
from typing import Any, ClassVar, Literal, TypedDict, final

_AddOperation = TypedDict(
    "_AddOperation",
    {"op": Literal["add"], "path": str, "value": object},
)
_RemoveOperation = TypedDict(
    "_RemoveOperation",
    {"op": Literal["remove"], "path": str},
)
_ReplaceOperation = TypedDict(
    "_ReplaceOperation",
    {"op": Literal["replace"], "path": str, "value": object},
)
_MoveOperation = TypedDict(
    "_MoveOperation",
    {"op": Literal["move"], "from": str, "path": str},
)
_CopyOperation = TypedDict(
    "_CopyOperation",
    {"op": Literal["copy"], "from": str, "path": str},
)
_TestOperation = TypedDict(
    "_TestOperation",
    {"op": Literal["test"], "path": str, "value": object},
)

type _PatchOperation = (
    _AddOperation
    | _RemoveOperation
    | _ReplaceOperation
    | _MoveOperation
    | _CopyOperation
    | _TestOperation
)


class Schema:
    def __init__(self, schema: dict[str, object] | bool) -> None: ...
    @property
    def value(self) -> dict[str, object] | bool: ...


@final
class ToolInput[Args, State = Any]:
    @property
    def args(self) -> Args: ...
    @property
    def state(self) -> State: ...
    @property
    def emit(self) -> Callable[[str, list[_PatchOperation]], None] | None: ...


@final
class ToolOutput[Result]:
    def __init__(
        self,
        output: Result,
        *,
        state_update: list[_PatchOperation] | None = None,
    ) -> None: ...


class Tool[Args, Result, State = Any]:
    description: ClassVar[str]
    parameters: ClassVar[Schema]
    def __init__(self) -> None: ...
    async def execute(self, input: ToolInput[Args, State], /) -> ToolOutput[Result]: ...

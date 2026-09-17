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
    """
    A JSON schema that describes the input parameters for a tool.
    """
    def __init__(self, schema: dict[str, object] | bool) -> None:
        """
        :param schema: The JSON schema or boolean value representing the tool's input parameters.
        :type schema: dict[str, object] | bool
        :return: None
        """
    @property
    def value(self) -> dict[str, object] | bool:
        """
        The JSON schema or boolean value representing the tool's input parameters
        as a dictionary or boolean.
        """

@final
class ToolInput[Args, State = Any]:
    """
    The input to a tool, including the arguments, state, and an optional emit function for emitting
    activity deltas.
    """
    @property
    def args(self) -> Args:
        """
        The arguments for the tool, as defined by the tool's input schema.
        """
    @property
    def state(self) -> State:
        """
        The current state of the agent, which can be used to maintain context.
        """
    @property
    def emit(self) -> Callable[[str, list[_PatchOperation]], None] | None:
        """
        An optional function that can be called to emit activity deltas.
        """

@final
class ToolOutput[Result]:
    """
    The output of a tool, including the result and an optional list of state update operations.
    """
    def __init__(
        self,
        output: Result,
        *,
        state_update: list[_PatchOperation] | None = None,
    ) -> None:
        """
        :param output: The result of the tool execution.
        :type output: Result
        :param state_update: An optional list of state update operations to be applied to the agent's state.
        :type state_update: list[_PatchOperation] | None
        :return: None
        """

class Tool[Args, Result, State = Any]:
    """
    A tool that can be executed by an agent, with defined input parameters and output results.
    """

    description: ClassVar[str]
    """The description of the tool, which is used by the agent to decide when to invoke it."""
    parameters: ClassVar[Schema]
    """The JSON schema that describes the input parameters for the tool."""
    def __init__(self) -> None: ...
    async def execute(self, input: ToolInput[Args, State], /) -> ToolOutput[Result]:
        """
        Execute the tool with the given input and return the output.

        :param input: The input to the tool, including arguments, state, and an optional emit function.
        :type input: ToolInput[Args, State]
        :return: The output of the tool execution, including the result and optional state update operations.
        :rtype: ToolOutput[Result]
        """

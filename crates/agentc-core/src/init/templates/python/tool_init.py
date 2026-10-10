from dataclasses import dataclass

from agentc_tools import Schema, Tool, ToolInput, ToolOutput


@dataclass
class {{ name_pascal }}Params:
    name: str


@dataclass
class {{ name_pascal }}Result:
    message: str


class {{ name_pascal }}(Tool[{{ name_pascal }}Params, {{ name_pascal }}Result]):
    description = "A tool."
    parameters = Schema({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "A name parameter.",
            },
        },
        "required": ["name"],
    })

    async def execute(
        self,
        input: ToolInput[{{ name_pascal }}Params],
    ) -> ToolOutput[{{ name_pascal }}Result]:
        return ToolOutput(
            {{ name_pascal }}Result(
                message=f"Hello, {input.args.name}!",
            ),
        )

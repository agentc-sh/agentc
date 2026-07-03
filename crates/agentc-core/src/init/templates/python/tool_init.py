from dataclasses import dataclass
from agentc_tdk import Tool, ToolInput, ToolOutput


@dataclass
class {{ name_pascal }}Params:
    name: str

@dataclass
class {{ name_pascal }}Result:
    message: str

class {{ name_pascal }}(Tool[{{ name_pascal }}Params, {{ name_pascal }}Result]):
    args = {{ name_pascal }}Params

    name = "{{ name_snake }}"
    description = "A tool."
    schema = {
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "A name parameter."
            }
        },
        "required": ["name"]
    }

    def execute(self, input: ToolInput[{{ name_pascal }}Params]) -> ToolOutput[{{ name_pascal }}Result]:
        return ToolOutput(
            output={{ name_pascal }}Result(
                message=f"Hello, {input.args.name}!"
            )
        )

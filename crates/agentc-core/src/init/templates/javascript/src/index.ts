import { Schema, Tool, type ToolInput, type ToolOutput } from 'agentc:tools';

export type {{ name_pascal }}Params = {
    name: string;
}

export type {{ name_pascal }}Result = {
    message: string;
}

export class {{ name_pascal }} extends Tool<{{ name_pascal }}Params, {{ name_pascal }}Result> {
    static readonly description = 'A tool.';

    static readonly parameters = new Schema({
        type: 'object',
        properties: {
            name: {
                type: 'string',
                description: 'A name parameter.',
            },
        },
        required: ['name'],
    });

    async execute(
        input: ToolInput<{{ name_pascal }}Params>,
    ): Promise<ToolOutput<{{ name_pascal }}Result>> {
        return {
            output: {
                message: `Hello, ${input.args.name}!`,
            },
        };
    }
}

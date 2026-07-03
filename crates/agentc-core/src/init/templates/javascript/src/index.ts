import { Tool, ToolInput, ToolOutput } from '@agentc-sh/tdk';

export type {{ name_pascal }}Params = {
    name: string;
}

export type {{ name_pascal }}Result = {
    message: string;
}

export const {{ name_snake }}: Tool<{{ name_pascal }}Params, {{ name_pascal }}Result> = {
    name: '{{ name_snake }}',
    description: 'A tool.',
    parameters: {
        type: 'object',
        properties: {
            name: {
                type: 'string',
                description: 'A name parameter.',
            },
        },
        required: ['name'],
    },
    async execute(input: ToolInput<{{ name_pascal }}Params>): Promise<ToolOutput<{{ name_pascal }}Result>> {
        return {
            output: {
                message: `Hello, ${input.args.name}!`,
            }
        }
    }
}

declare module 'agentc:tools' {
  export type JsonPointer = `/${string}`

  export type PatchOperation =
    | { op: 'add'; path: JsonPointer; value: unknown }
    | { op: 'remove'; path: JsonPointer }
    | { op: 'replace'; path: JsonPointer; value: unknown }
    | { op: 'move'; from: JsonPointer; path: JsonPointer }
    | { op: 'copy'; from: JsonPointer; path: JsonPointer }
    | { op: 'test'; path: JsonPointer; value: unknown }

  export type PatchDocument = Array<PatchOperation>

  export interface ActivityDelta {
    activity_type: string
    patch: PatchDocument
  }

  export class Schema {
    constructor(schema: Record<string, unknown>)
    readonly value: Record<string, unknown>
  }

  export class ToolInput<Args, State = unknown> {
    private constructor()
    readonly args: Args
    readonly state: State
    readonly emit?: (delta: ActivityDelta) => void
  }

  export type ToolOutput<Result> = {
    output: Result
    state_update?: PatchDocument
  }

  export abstract class Tool<Args, Result, State = unknown> {
    static readonly description: string
    static readonly parameters: Schema
    abstract execute(input: ToolInput<Args, State>): Promise<ToolOutput<Result>>
  }
}

declare module 'agentc:tools' {
  /**
   * A JSON Pointer identifying the target of a patch operation. Escape `~` as
   * `~0` and `/` as `~1` within a path segment.
   */
  export type JsonPointer = `/${string}`

  /**
   * One JSON Patch operation. `add`, `replace`, and `test` carry `value`;
   * `move` and `copy` carry `from`. `path` identifies the destination or the
   * value to remove or test.
   */
  export type PatchOperation =
    | { op: 'add'; path: JsonPointer; value: unknown }
    | { op: 'remove'; path: JsonPointer }
    | { op: 'replace'; path: JsonPointer; value: unknown }
    | { op: 'move'; from: JsonPointer; path: JsonPointer }
    | { op: 'copy'; from: JsonPointer; path: JsonPointer }
    | { op: 'test'; path: JsonPointer; value: unknown }

  /** An ordered sequence of JSON Patch operations applied as one patch. */
  export type PatchDocument = Array<PatchOperation>

  /** A JSON Patch update for a named activity. */
  export interface ActivityDelta {
    /** The activity type whose content receives this delta. */
    activity_type: string
    /** The ordered patch operations for the activity content. */
    patch: PatchDocument
  }

  /** A Draft 2020-12 JSON Schema describing a tool's input parameters. */
  export class Schema {
    /**
     * Creates a tool parameter schema.
     * @param schema - The JSON Schema document to validate.
     * @throws If the document is invalid under Draft 2020-12.
     */
    constructor(schema: Record<string, unknown>)

    /** The schema document supplied at construction. */
    readonly value: Record<string, unknown>
  }

  /**
   * The arguments, state snapshot, and optional activity emitter supplied to a
   * tool invocation.
   * @typeParam Args - The tool's input argument shape.
   * @typeParam State - The state shape supplied by the selected graph.
   */
  export class ToolInput<Args, State = unknown> {
    private constructor()

    /** The arguments supplied for this invocation. */
    readonly args: Args
    /** The graph state snapshot supplied for this invocation. */
    readonly state: State
    /**
     * Emits an activity delta when an emitter is available. Use optional
     * chaining when emitting, such as `input.emit?.(delta)`.
     */
    readonly emit?: (delta: ActivityDelta) => void
  }

  /**
   * The tool result and optional state patch returned from an invocation.
   * @typeParam Result - The shape of the returned result.
   */
  export type ToolOutput<Result> = {
    /** The result returned by the tool. */
    output: Result
    /**
     * An optional JSON Patch document interpreted by the selected graph's
     * state-update contract.
     */
    state_update?: PatchDocument
  }

  /**
   * Base class for tools implemented by a TypeScript package.
   * @typeParam Args - The tool's input argument shape.
   * @typeParam Result - The shape of the returned result.
   * @typeParam State - The state shape supplied by the selected graph.
   */
  export abstract class Tool<Args, Result, State = unknown> {
    /** The model-facing description used when the manifest supplies none. */
    static readonly description?: string
    /** The JSON Schema describing accepted input arguments. */
    static readonly parameters: Schema

    /**
     * Executes the tool with the supplied invocation input.
     * @param input - The arguments, graph state, and optional activity emitter.
     * @returns The tool result and optional state patch.
     */
    abstract execute(input: ToolInput<Args, State>): Promise<ToolOutput<Result>>
  }
}

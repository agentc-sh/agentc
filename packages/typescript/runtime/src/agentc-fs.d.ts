declare module 'node:fs/promises' {
  export interface ReadFileOptions {
    encoding?: string
  }

  export interface WriteFileOptions {
    mode?: number
  }

  export interface MkdirOptions {
    recursive?: boolean
    mode?: number
  }

  export interface ReaddirOptions {
    withFileTypes?: boolean
    recursive?: boolean
  }

  export interface RmOptions {
    recursive?: boolean
    force?: boolean
  }

  export interface RmdirOptions {
    recursive?: boolean
  }

  export const constants: {
    F_OK: number
    R_OK: number
    W_OK: number
    X_OK: number
  }

  export function access(path: string, mode?: number): Promise<void>
  export function open(
    path: string,
    flags?: string,
    mode?: number,
  ): Promise<FileHandle>
  /** Resolves to a string when an encoding is given, and to a `Uint8Array` otherwise. */
  export function readFile(
    path: string,
    options?: string | ReadFileOptions,
  ): Promise<unknown>
  export function writeFile(
    path: string,
    data: string | Uint8Array,
    options?: string | WriteFileOptions,
  ): Promise<void>
  export function rename(oldPath: string, newPath: string): Promise<void>
  /** Resolves to `Dirent[]` when `withFileTypes` is set, and to `string[]` otherwise. */
  export function readdir(
    path: string,
    options?: ReaddirOptions,
  ): Promise<unknown[]>
  export function mkdir(path: string, options?: MkdirOptions): Promise<string>
  export function mkdtemp(prefix: string): Promise<string>
  export function rm(path: string, options?: RmOptions): Promise<void>
  export function rmdir(path: string, options?: RmdirOptions): Promise<void>
  export function stat(path: string): Promise<Stats>
  export function lstat(path: string): Promise<Stats>
  export function chmod(path: string, mode: number): Promise<void>
  export function symlink(
    target: string,
    path: string,
    type?: string,
  ): Promise<void>
}

declare module 'fs/promises' {
  export * from 'node:fs/promises'
}

declare module 'node:fs' {
  import type {
    MkdirOptions,
    ReadFileOptions,
    ReaddirOptions,
    RmOptions,
    RmdirOptions,
    WriteFileOptions,
  } from 'node:fs/promises'

  export const promises: typeof import('node:fs/promises')

  export const constants: {
    F_OK: number
    R_OK: number
    W_OK: number
    X_OK: number
  }

  export function accessSync(path: string, mode?: number): void
  export function mkdirSync(path: string, options?: MkdirOptions): string
  export function mkdtempSync(prefix: string): string
  /** Returns `Dirent[]` when `withFileTypes` is set, and `string[]` otherwise. */
  export function readdirSync(path: string, options?: ReaddirOptions): unknown[]
  /** Returns a string when an encoding is given, and a `Uint8Array` otherwise. */
  export function readFileSync(
    path: string,
    options?: string | ReadFileOptions,
  ): unknown
  export function rmdirSync(path: string, options?: RmdirOptions): void
  export function rmSync(path: string, options?: RmOptions): void
  export function statSync(path: string): Stats
  export function lstatSync(path: string): Stats
  export function writeFileSync(
    path: string,
    data: string | Uint8Array,
    options?: string | WriteFileOptions,
  ): void
  export function chmodSync(path: string, mode: number): void
  export function renameSync(oldPath: string, newPath: string): void
  export function symlinkSync(target: string, path: string, type?: string): void
}

declare module 'fs' {
  export * from 'node:fs'
}

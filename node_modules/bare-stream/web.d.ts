import { Writable } from '.'

export interface ReadableStreamDefaultReader {
  read(): Promise<{ value: unknown; done: boolean }>
  cancel(reason?: unknown): Promise<void>
}

export class ReadableStreamDefaultReader {
  constructor(stream: ReadableStream)
}

export interface ReadableStreamDefaultController {
  readonly desiredSize: number

  enqueue(data: unknown): void
  close(): void
  error(error?: unknown): void
}

export class ReadableStreamDefaultController {
  constructor(stream: ReadableStream)
}

export interface UnderlyingSource<S extends ReadableStream = ReadableStream> {
  start?(this: S, controller: ReadableStreamDefaultController): void
  pull?(this: S, controller: ReadableStreamDefaultController): void
}

export interface CustomQueuingStrategy {
  highWaterMark?: number
  size?: (chunk: unknown) => number
}

export interface ReadableStream extends AsyncIterable<unknown> {
  getReader(): ReadableStreamDefaultReader
  cancel(reason?: unknown): Promise<void>
  pipeTo(destination: Writable): Promise<void>
}

export class ReadableStream {
  constructor(
    underlyingSource?: UnderlyingSource,
    queuingStrategy?: CustomQueuingStrategy
  )

  static from(
    iterable: unknown | unknown[] | AsyncIterable<unknown>
  ): ReadableStream
}

export interface QueuingStrategyOptions {
  highWaterMark?: number
}

interface QueuingStrategy {
  readonly highWaterMark: number

  size(chunk: unknown): number
}

declare class QueuingStrategy {
  constructor(opts?: QueuingStrategyOptions)
}

export { type QueuingStrategy }

export class CountQueuingStrategy extends QueuingStrategy {}

export class ByteLengthQueuingStrategy extends QueuingStrategy {}

export function isReadableStream(value: unknown): value is ReadableStream

export function isReadableStreamDisturbed(stream: ReadableStream): boolean

--!strict

--[=[
	@class Task

	Built-in task scheduler, thread spawning & multi-core worker threads.
]=]

export type ParallelTask = {
	-- Unique worker ID
	Id: number,

	-- Sends values into the worker
	Push: (self: ParallelTask, ...any) -> (),

	-- Receives values from the worker
	Pop: (self: ParallelTask) -> ...any,

	-- Closes the selected thread.
	Close: (self: ParallelTask) -> (),
}

local task = {}

--[=[
	@class ParallelTask

	Represents a worker running in a separate Lua VM
	on its own CPU thread.

	Each worker has a unique `Id`.

	Communication is done using `:Push(...)` and `:Pop()`.

	⚠️ Workers do not share state with the main VM.
	Only primitive values and tables are supported.
]=]
local ParallelTask = {}

--[=[
	@within ParallelTask
	Unique numeric identifier for this worker.
]=]
ParallelTask.Id = 0

--[=[
	@within ParallelTask

	Sends values into the worker thread.

	Supported types:
	• nil
	• boolean
	• number
	• string
	• tables (recursive)

	Unsupported:
	• functions
	• userdata
	• threads
]=]
function ParallelTask:Push(...: any) end

--[=[
	@within ParallelTask

	Attempts to receive values sent back from the worker.

	Errors if no values are available.
]=]
function ParallelTask:Pop(): ...any
	return nil :: any
end

--[=[
	@within Task

	Creates a new multi-core worker thread.

	The provided string is executed inside a new,
	isolated Lua VM on a separate OS thread.

	Each worker has a unique `Id` property.

	Inside the worker:
	• `task.pop()` receives values
	• `task.push(...)` sends values back

	@param script Lua source code string
	@return ParallelTask handle
]=]
function task.parallel(script: string): ParallelTask
	return nil :: any
end

-- Worker-side functions

--[=[
	@within Task

	Receives values inside a worker thread.

	⚠️ Only valid inside a `task.parallel` worker.
]=]
function task.pop(): ...any
	return nil :: any
end

--[=[
	@within Task

	Sends values from inside a worker thread back
	to the main thread.

	⚠️ Only valid inside a `task.parallel` worker.
]=]
function task.push(...: any) end

-- Scheduler functions

function task.cancel(thread: thread) end

function task.defer<T...>(functionOrThread: thread | (T...) -> ...any, ...: T...): thread
	return nil :: any
end

function task.delay<T...>(
	duration: number,
	functionOrThread: thread | (T...) -> ...any,
	...: T...
): thread
	return nil :: any
end

function task.spawn<T...>(functionOrThread: thread | (T...) -> ...any, ...: T...): thread
	return nil :: any
end

function task.wait(duration: number?): number
	return nil :: any
end

return task

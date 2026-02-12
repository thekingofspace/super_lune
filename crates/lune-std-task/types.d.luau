--[=[
	@class Task

	Built-in task scheduler, thread spawning & multi-core worker threads.

	### Example usage

	```lua
	local task = require("@lune/task")

	-- Waiting for a certain amount of time
	task.wait(1)
	print("Waited for one second")

	-- Running a task after a given amount of time
	task.delay(2, function()
		print("Ran after two seconds")
	end)

	-- Spawning a new task that runs concurrently
	task.spawn(function()
		print("Running instantly")
		task.wait(1)
		print("One second passed inside the task")
	end)

	print("Running after task.spawn yields")

	-- True multi-core worker
	local worker = task.parallel([[
		while true do
			local a, b = task.pop()
			task.push(a * b)
		end
	]])

	worker:Push(4, 5)
	task.wait(0.1)
	print(worker:Pop()) -- 20
	```
]=]

export type ParallelTask = {
	Push: (self: ParallelTask, ...any) -> (),
	Pop: (self: ParallelTask) -> ...any,
}

local task = {}

--[=[
	@class ParallelTask

	Represents a worker running in a separate Lua VM
	on its own CPU thread.

	Communication is done using `:Push(...)` and `:Pop()`.

	⚠️ Workers do not share state with the main VM.
	Only primitive values and tables are supported.
]=]
local ParallelTask = {}

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

	Inside the worker:
	• `task.pop()` receives values
	• `task.push(...)` sends values back

	@param script Lua source code string
	@return ParallelTask handle
]=]
function task.parallel(script: string): ParallelTask
	return nil :: any
end

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

--[=[
	@within Task

	Stops a currently scheduled thread from resuming.

	@param thread The thread to cancel
]=]
function task.cancel(thread: thread) end

--[=[
	@within Task

	Defers a thread or function to run at the end of the current task queue.

	@param functionOrThread The function or thread to defer
	@return The thread that will be deferred
]=]
function task.defer<T...>(functionOrThread: thread | (T...) -> ...any, ...: T...): thread
	return nil :: any
end

--[=[
	@within Task

	Delays a thread or function to run after `duration` seconds.

	@param duration Delay duration in seconds
	@param functionOrThread The function or thread to delay
	@return The thread that will be delayed
]=]
function task.delay<T...>(
	duration: number,
	functionOrThread: thread | (T...) -> ...any,
	...: T...
): thread
	return nil :: any
end

--[=[
	@within Task

	Instantly runs a thread or function.

	If the spawned task yields, the thread that spawned the task
	will resume, letting the spawned task run in the background.

	@param functionOrThread The function or thread to spawn
	@return The thread that was spawned
]=]
function task.spawn<T...>(functionOrThread: thread | (T...) -> ...any, ...: T...): thread
	return nil :: any
end

--[=[
	@within Task

	Waits for *at least* the given amount of time.

	The minimum wait time possible when using `task.wait`
	is limited by the underlying OS sleep implementation.

	@param duration The amount of time to wait
	@return The exact amount of time waited
]=]
function task.wait(duration: number?): number
	return nil :: any
end

return task

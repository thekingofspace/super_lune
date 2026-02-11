--!strict

--[=[
	@class MemoryBlock

	A fixed-size memory allocation returned from `memory.malloc`.

	Memory blocks have fixed capacity. Writing beyond capacity
	will throw a fatal error.

	Memory blocks can be manually freed or scheduled for cleanup.
]=]
export type MemoryBlock = {
	--[=[
		Writes data into the memory block.

		This copies the data into internal storage.
		Throws an error if capacity is exceeded.
	]=]
	Write: (self: MemoryBlock, data: any) -> (),

	--[=[
		Reads the contents of the memory block.
	]=]
	Read: (self: MemoryBlock) -> any,

	--[=[
		Frees the memory block immediately.

		After calling this, the memory block becomes invalid.
	]=]
	Free: (self: MemoryBlock) -> (),

	--[=[
		Schedules this memory block for cleanup.

		If `ttl` is provided, the block will expire after
		that many seconds. Defaults to 60 seconds.
	]=]
	Schedule: (self: MemoryBlock, ttl: number?) -> (),

	--[=[
		Returns the current size (number of bytes written).
	]=]
	Size: (self: MemoryBlock) -> number,

	--[=[
		Returns the fixed capacity of this block.
	]=]
	Capacity: (self: MemoryBlock) -> number,
}

--[=[
	@class Memory

	Low-level memory management library.

	Provides fixed-size manual allocation with optional
	scheduling and cleanup behavior.
]=]
local memory = {}

--[=[
	@within Memory
	@tag must_use

	Allocates a fixed-size memory block.

	Throws an error if `size` is zero.

	### Example

	```lua
	local memory = require("@lune/memory")

	local buf = memory.malloc(32)
	buf:Write("hello")
	print(buf:Read())
	```
]=]
function memory.malloc(size: number): MemoryBlock
	return nil :: any
end

--[=[
	@within Memory

	Iterates through all allocated memory blocks.

	The given callback will receive each MemoryBlock.
	If the callback returns `true`, the block will be cleaned.

	### Example

	```lua
	memory.Clean(function(block)
		if block:Size() == 0 then
			return true
		end
		return false
	end)
	```
]=]
function memory.Clean(callback: (block: MemoryBlock) -> boolean): ()
	return nil :: any
end

return memory

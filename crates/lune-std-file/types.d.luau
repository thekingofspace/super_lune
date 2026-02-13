--!nocheck

--[=[
	@type FilePrimitive
	@within File

	Primitive values supported by the File library.
]=]
export type FilePrimitive = nil | boolean | number | string

--[=[
	@type FileValue
	@within File

	Values that can be written to a File.

	Supports:
	* nil
	* boolean
	* number
	* string
	* nested tables containing supported values
]=]
export type FileValue = FilePrimitive | { [FileValue]: FileValue }

--[=[
	@class File

	Binary memory-backed file object.

	This object allows writing Lua values at specific byte
	positions and later reading them back.

	Data can be serialized into a raw byte string and written
	to disk using the `fs` library.

	### Example usage

	```lua
	local file = require("@lune/file")

	local f = file.new()

	f:write(0, {
		name = "Raikin",
		level = 42,
	})

	local data = f:read(0)

	print(data.name) -- Raikin

	local raw = f:serialize()
	```
]=]
export type File = {
	--[=[
		Writes a value to the file at a given byte position.

		This is a **raw memory write** operation.

		If the position overlaps with existing serialized data,
		that data may become corrupted. This method is faster
		and intended for advanced usage where byte layout is
		managed manually.

		If the position exceeds the current file size,
		the internal buffer will automatically expand.

		@param position The byte offset to write at
		@param value The value to write
	]=]
	write: (self: File, position: number, value: FileValue) -> (),

	--[=[
		Reads a value from a given byte position.

		This is a **raw memory read** operation.

		If the position does not contain valid serialized data,
		nil may be returned or an error may occur if the data
		has been corrupted by overlapping writes.

		For structured access, consider using `safeRead`.

		@param position The byte offset to read from
		@return The deserialized value
	]=]
	read: (self: File, position: number) -> FileValue,

	--[=[
		Writes a value into a structured slot.

		This is a **safe indexed write** operation.

		Values written using `safeWrite` will never overlap
		or corrupt other stored values.

		This method is slower than `write` because it
		rebuilds the internal structure to maintain safety.

		@param slot The logical slot index
		@param value The value to write
	]=]
	safeWrite: (self: File, slot: number, value: FileValue) -> (),

	--[=[
		Reads a value from a structured slot.

		This is a **safe indexed read** operation.

		If the slot does not exist, nil will be returned.

		@param slot The logical slot index
		@return The stored value or nil
	]=]
	safeRead: (self: File, slot: number) -> FileValue,
	--[=[
		Serializes the file's internal buffer into a raw binary string.

		This string may be written to disk using `fs.writeFile`
		or transmitted over the network.

		@return Raw binary data
	]=]
	serialize: (self: File) -> string,
}

--[=[
	@class FileLibrary

	Module entry for `@lune/file`.
]=]
local file = {}

--[=[
	@within FileLibrary
	@tag must_use

	Creates a new in-memory File object.

	@return A new File instance
]=]
function file.new(): File
	return nil :: any
end

--[=[
	@within FileLibrary
	@tag must_use

	Deserializes raw binary data into a File object.

	The input must be a string previously produced by `File:serialize()`.

	@param data Raw binary data
	@return A reconstructed File instance
]=]
function file.deserialize(data: string): File
	return nil :: any
end

return file

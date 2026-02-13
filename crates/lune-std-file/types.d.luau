--!nocheck

--[=[
	@type FilePrimitive
	@within File

	Primitive values supported by the SAFE region.
]=]
export type FilePrimitive = nil | boolean | number | string

--[=[
	@type FileValue
	@within File

	Values that can be written using safeWrite.

	Supports:
	* nil
	* boolean
	* number
	* string
]=]
export type FileValue = FilePrimitive

--[=[
	@type FileTypeId
	@within File

	Type identifiers used for raw typed writes.
]=]
export type FileTypeId = number -- provided by file.types (i8, u8, i16, etc.)

--[=[
	@class FileTypes
	@within File

	Primitive binary types available for raw memory writes.
]=]
export type FileTypes = {
	i8: number,
	u8: number,
	i16: number,
	u16: number,
	i32: number,
	u32: number,
	i64: number,
	u64: number,
	f32: number,
	f64: number,
	bool: number,
	string: number,
}

--[=[
	@class File

	Binary memory-backed file object.

	Supports two systems:

	1) Raw typed writes (fast, manual layout control)
	2) Safe slot writes (structured, non-overlapping)
]=]
export type File = {
	--[=[
		Writes a typed value at a specific byte offset.

		This is a raw binary write and requires a type id
		from `file.types`.

		Example:
		```lua
		f:write(0, file.types.i32, 123)
		```

		@param position Byte offset
		@param typeId Type from file.types
		@param value Value matching that type
	]=]
	write: (self: File, position: number, typeId: FileTypeId, value: any) -> (),

	--[=[
		Reads a typed value from a byte offset.

		The type is inferred from the stored type header.

		@param position Byte offset
		@return The decoded value
	]=]
	read: (self: File, position: number) -> any,

	--[=[
		Writes a value into a structured safe slot.

		Values stored here will never overlap.

		@param slot Logical slot id
		@param value Primitive value
	]=]
	safeWrite: (self: File, slot: number, value: FileValue) -> (),

	--[=[
		Reads a value from a structured safe slot.

		@param slot Logical slot id
		@return Stored value or nil
	]=]
	safeRead: (self: File, slot: number) -> FileValue,

	--[=[
		Serializes the file buffer into raw binary data.

		@return Binary string
	]=]
	serialize: (self: File) -> string,
}

--[=[
	@class FileLibrary

	Module entry for `@lune/file`.
]=]
export type FileLibrary = {
	new: () -> File,
	deserialize: (data: string) -> File,

	-- Available binary primitive types
	types: FileTypes,
}

local file = {} :: FileLibrary

function file.new(): File
	return nil :: any
end

function file.deserialize(data: string): File
	return nil :: any
end

return file

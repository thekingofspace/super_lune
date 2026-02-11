--[=[
	@class ObjectId
	@within Mongo

	Represents a MongoDB ObjectId value.
]=]
export type ObjectId = {
	-- Returns the 24-character hexadecimal string representation.
	toHex: (self: ObjectId) -> string,
}

--[=[
	@class DateTime
	@within Mongo

	Represents a MongoDB BSON DateTime value.
]=]
export type DateTime = {
	-- Returns the timestamp in milliseconds.
	toMillis: (self: DateTime) -> number,
}

--[=[
	@class MongoClient
	@within Mongo

	A MongoDB client connection instance.

	Returned from `mongo.connect()`.
]=]
export type MongoClient = {
	--[=[
		Gets a database handle by name.

		@param name The database name
		@return MongoDatabase
	]=]
	database: (self: MongoClient, name: string) -> MongoDatabase,
}

--[=[
	@class MongoDatabase
	@within Mongo

	A MongoDB database handle.
]=]
export type MongoDatabase = {
	--[=[
		Gets a collection handle by name.

		@param name The collection name
		@return MongoCollection
	]=]
	collection: (self: MongoDatabase, name: string) -> MongoCollection,
}

--[=[
	@class MongoCollection
	@within Mongo

	A MongoDB collection handle.
]=]
export type MongoCollection = {
	--[=[
		Inserts a single document.

		@param document The document to insert
		@return ObjectId The generated document ID
	]=]
	insertOne: (
		self: MongoCollection,
		document: { [string]: any }
	) -> ObjectId,

	--[=[
		Inserts multiple documents.

		@param documents Array of documents
	]=]
	insertMany: (
		self: MongoCollection,
		documents: { { [string]: any } }
	) -> (),

	--[=[
		Finds a single document matching the filter.

		@param filter A MongoDB filter object
		@return The matching document or nil
	]=]
	findOne: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> { [string]: any }?,

	--[=[
		Finds all documents matching the filter.

		@param filter A MongoDB filter object
		@return Array of matching documents
	]=]
	find: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> { { [string]: any } },

	--[=[
		Updates a single document matching the filter.

		@param filter The filter object
		@param update The update document (ex: { ["$set"] = {...} })
	]=]
	updateOne: (
		self: MongoCollection,
		filter: { [string]: any },
		update: { [string]: any }
	) -> (),

	--[=[
		Deletes a single document matching the filter.

		@param filter The filter object
	]=]
	deleteOne: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> (),

	--[=[
		Counts the number of documents matching the filter.

		@param filter The filter object
		@return number
	]=]
	countDocuments: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> number,
}

--[=[
	@class MongoObjectAPI
	@within Mongo

	Helper constructors for BSON-specific types.
]=]
export type MongoObjectAPI = {
	--[=[
		Creates a new ObjectId.

		@return ObjectId
	]=]
	objectId: () -> ObjectId,

	--[=[
		Creates a new BSON DateTime set to now.

		@return DateTime
	]=]
	date: () -> DateTime,
}

--[=[
	@class Mongo

	Built-in MongoDB driver for Lune.

	Provides asynchronous access to MongoDB servers.

	### Example

	```lua
	local mongo = require("@lune/mongo")

	local client = mongo.connect("mongodb://localhost:27017")
	local db = client:database("test")
	local users = db:collection("users")

	local id = users:insertOne({
		name = "Raikin",
		createdAt = mongo.object.date()
	})
	

	local user = users:findOne({ _id = id })

	print(user._id:toHex())
	print(user.createdAt:toMillis())

	local total = users:countDocuments({})
	print(total)
	```
]=]
local mongo = {}

--[=[
	@within Mongo

	Connects to a MongoDB server.

	@param uri MongoDB connection string
	@return MongoClient
]=]
function mongo.connect(uri: string): MongoClient
	return nil :: any
end

-- Helper object API
mongo.object = {} :: MongoObjectAPI

return mongo

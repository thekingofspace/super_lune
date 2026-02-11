--[=[
	@class ObjectId
	@within Mongo

	Represents a MongoDB ObjectId value.
]=]
export type ObjectId = {
	toHex: (self: ObjectId) -> string,
}

--[=[
	@class DateTime
	@within Mongo

	Represents a MongoDB BSON DateTime value.
]=]
export type DateTime = {
	toMillis: (self: DateTime) -> number,
}

--[=[
	@class MongoClient
	@within Mongo

	A MongoDB client connection instance.
]=]
export type MongoClient = {
	database: (self: MongoClient, name: string) -> MongoDatabase,
}

--[=[
	@class MongoDatabase
	@within Mongo

	A MongoDB database handle.
]=]
export type MongoDatabase = {
	collection: (self: MongoDatabase, name: string) -> MongoCollection,
}

--[=[
	@class MongoFindOptions
	@within Mongo

	Optional configuration for find / findOne.
]=]
export type MongoFindOptions = {
	sort: { [string]: number }?,        -- 1 or -1
	limit: number?,
	skip: number?,
	projection: { [string]: number }?,
}

--[=[
	@class MongoUpdateOptions
	@within Mongo

	Optional configuration for updateOne / updateMany.
]=]
export type MongoUpdateOptions = {
	upsert: boolean?,
}

--[=[
	@class MongoCollection
	@within Mongo

	A MongoDB collection handle.
]=]
export type MongoCollection = {

	insertOne: (
		self: MongoCollection,
		document: { [string]: any }
	) -> ObjectId,

	insertMany: (
		self: MongoCollection,
		documents: { { [string]: any } }
	) -> (),

	findOne: (
		self: MongoCollection,
		filter: { [string]: any },
		options: MongoFindOptions?
	) -> { [string]: any }?,

	find: (
		self: MongoCollection,
		filter: { [string]: any },
		options: MongoFindOptions?
	) -> { { [string]: any } },

	updateOne: (
		self: MongoCollection,
		filter: { [string]: any },
		update: { [string]: any },
		options: MongoUpdateOptions?
	) -> (),

	updateMany: (
		self: MongoCollection,
		filter: { [string]: any },
		update: { [string]: any },
		options: MongoUpdateOptions?
	) -> (),

	deleteOne: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> (),

	deleteMany: (
		self: MongoCollection,
		filter: { [string]: any }
	) -> (),

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
	objectId: () -> ObjectId,
	date: () -> DateTime,
}

--[=[
	@class Mongo

	Built-in MongoDB driver for Lune.

	Supports sorting, limits, skip, projection and upsert.
]=]
local mongo = {}

function mongo.connect(uri: string): MongoClient
	return nil :: any
end

mongo.object = {} :: MongoObjectAPI

return mongo

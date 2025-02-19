package core

func GetDatabaseType() DBType {
	switch Get("database.type") {
	case string(DBTypeSqlite):
		return DBTypeSqlite
	case string(DBTypeNone):
		return DBTypeNone
	default:
		panic(panicMissConfig("database.type", Get("database.type")))
	}

}

func GetStorageType() StorageType {
	switch Get("storage.type") {
	case string(StorageLocalDirectory):
		return StorageLocalDirectory
	case string(StorageNone):
		return StorageNone
	default:
		panic(panicMissConfig("storage.type", Get("storage.type")))
	}

}

func GetSecureType() SecureType {
	switch Get("secure.type") {
	case string(Os):
		return Os
	case string(Static):
		return Static
	default:
		panic(panicMissConfig("secure.type", Get("secure.type")))
	}

}

func GetIssuerType() IssuerType {
	switch Get("issuer.type") {
	case string(IssuerFromDB):
		return IssuerFromDB
	case string(IssuerFromEnv):
		return IssuerFromEnv
	default:
		panic(panicMissConfig("issuer.type", Get("issuer.type")))
	}
}

func IsIssuerStatic() bool {
	return GetIssuerType() == IssuerFromEnv
}

func IsIssuerDB() bool {
	return GetIssuerType() == IssuerFromDB
}

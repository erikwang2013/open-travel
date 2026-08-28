# e-cat-data

Unified data access traits for the e-cat ecosystem.

## Traits

| Trait | Purpose | Implementations |
|-------|---------|----------------|
| `RdbmsClient` | SQL databases | SQLx, ClickHouse, QuestDB |
| `Cache` | Key-value cache | Redis, Memcached |
| `GraphClient` | Graph databases | Neo4j, ArangoDB, NebulaGraph |
| `SearchClient` | Search engines | Elasticsearch, OpenSearch |
| `TsdbClient` | Time-series DBs | InfluxDB, IoTDB |

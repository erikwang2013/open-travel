# e-cat-data

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


Unified data access traits for the e-cat ecosystem.

## Traits

| Trait | Purpose | Implementations |
|-------|---------|----------------|
| `RdbmsClient` | SQL databases | SQLx, ClickHouse, QuestDB |
| `Cache` | Key-value cache | Redis, Memcached |
| `GraphClient` | Graph databases | Neo4j, ArangoDB, NebulaGraph |
| `SearchClient` | Search engines | Elasticsearch, OpenSearch |
| `TsdbClient` | Time-series DBs | InfluxDB, IoTDB |

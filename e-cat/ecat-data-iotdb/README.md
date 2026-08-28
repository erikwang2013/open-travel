# ecat-data-iotdb

Apache IoTDB time-series database client for e-cat (REST v2 API).

Writes use the REST v2 `insertTablet` endpoint: one tablet per `DataPoint`
with `device`, `timestamps`, `measurements`, `data_types` and a 2-D `values`
array. Tags are not representable in `insertTablet` and are ignored on write.

Part of the [e-cat](https://github.com/erik/e-cat) ecosystem.

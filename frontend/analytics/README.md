# Stellar Network Topology Visualizer

A standalone React + Three.js SPA for inspecting multi-cluster SCP quorum topology. The graph uses instanced WebGL spheres for validators and one batched line buffer for quorum links. This keeps the scene object count constant while the graph updates.

## Run It

From `frontend/analytics`:

```bash
npm install
npm run dev
```

Open the Vite URL printed by the command. The default source is the operator WebSocket endpoint:

```text
/api/v1/quorum/topology/stream
```

The operator stream sends `QuorumTopologyResponse` snapshots every five seconds. The frontend also accepts individual JSON `ScpMessage` records. For a real Kafka topic, run `npm run stream:kafka`; the KafkaJS bridge consumes `KAFKA_TOPIC` from `KAFKA_BROKERS` and broadcasts JSON messages to browser clients over WebSocket.

## Mock Load Test

Start the deterministic 500-node / 2,000-edge stream in a second terminal:

```bash
npm run mock:stream
```

Choose **Mock Kafka stream** in the app, or open the app with `?source=mock`. Customize the workload when measuring hardware:

```bash
node scripts/mock-kafka-stream.mjs --serve --nodes 500 --edges 2000 --interval 120
```

For a real topic bridge:

```bash
KAFKA_BROKERS=localhost:9092 KAFKA_TOPIC=stellar-scp-messages npm run stream:kafka
```

Set `KAFKA_FROM_BEGINNING=true` to replay retained messages. The bridge expects JSON values, matching the existing JSON serialization path in the operator pipeline.

The generator sends one initial snapshot and then individual SCP messages containing phase, ballot, TPS, ledger time, and quorum-set updates. Without `--serve`, it writes newline-delimited JSON records to stdout for replay or piping into a Kafka producer.

## Data Mapping

Snapshot nodes use the existing operator fields: `id`, `full_id`, `phase`, `is_critical`, `threshold`, and `stalled`. Individual messages use the fields in `schemas/scp_message.proto`. TPS and ledger time are read from `metrics.tps`, `metrics.ledger_time_ms`, or equivalent snake/camel-case fields and metadata. The current repository SCP schemas do not define those two measurements, so live producers must enrich the message or metadata for the inspector to show them; the mock stream includes both.

Node colors indicate health: green is synced, amber is degraded, and red is falling behind or unknown. Click a node to inspect cluster, SCP phase, ballot, TPS, ledger time, and quorum threshold. OrbitControls provides drag-to-orbit, scroll-to-zoom, and pan interaction.

## Checks

```bash
npm test
npm run build
```

The model tests exercise both snapshot and message ingestion. The browser performance target is validated with the mock harness and browser devtools or a production preview build; the renderer avoids per-edge/per-node React elements and limits device pixel ratio to reduce GPU pressure.

## Fee Estimator Explorer

The **Fee explorer** view (top bar toggle) visualizes ledger base fee trends and recommends priority fee-bump rates in real time.

- **Time-series chart** shows average ledger base fees over `1h` / `6h` / `24h` / `7d` windows. The SVG chart subscribes to the fee feed directly and keeps its own state, so live ticks update the chart without re-rendering the surrounding explorer or topology UI.
- **Priority tiers** (Low / Medium / High) are recomputed from the live base fee and a congestion factor (`recent mean ÷ baseline mean`). Surge spikes raise all recommended `maxFee` values.
- **Fee calculator** projects classic inclusion fees plus Soroban resource fees (CPU instructions, read/write bytes, events) and applies the selected tier multiplier to suggest a fee-bump `maxFee`.

Run the deterministic mock fee stream (24h of history plus scheduled spike hours) in a second terminal:

```bash
npm run mock:fees
```

Then open **Fee explorer** (defaults to the mock stream). Customize with:

```bash
node scripts/mock-fee-stream.mjs --serve --history-hours 48 --spike-hours 9,21 --interval 1000
```

Without `--serve` the generator emits newline-delimited JSON to stdout for replay. Live mode reads fee-enriched frames from `/api/v1/quorum/topology/stream`; when a frame carries no fee field, the feed infers a base fee from `tps`. The estimator model (`src/fees/feeModel.js`) and its tests (`npm test`) validate that historical fee spike data moves the congestion level and recommended tiers.

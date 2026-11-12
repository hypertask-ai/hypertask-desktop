import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const BUDGET_MS = 300;

export function readStartupSamples(report) {
  const results = report?.results;
  if (!Array.isArray(results) || results.length === 0) {
    throw new Error('Hyperfine report has no results');
  }
  const result = results[0];
  if (report.schema_version >= 2) {
    const samples = result?.measurements;
    const medianMs = result?.summary?.time_wall_clock?.median * 1000;
    if (!Array.isArray(samples) || samples.length === 0) {
      throw new Error('Hyperfine 2 report has no measurements');
    }
    return {
      samplesMs: samples.map((sample) => sample.time_wall_clock.value * 1000),
      medianMs,
    };
  }
  if (!Array.isArray(result.times) || result.times.length === 0) {
    throw new Error('Hyperfine 1 report has no times');
  }
  return {
    samplesMs: result.times.map((seconds) => seconds * 1000),
    medianMs: result.median * 1000,
  };
}

export function reportStartup(report, budgetMs = BUDGET_MS) {
  const { samplesMs, medianMs } = readStartupSamples(report);
  const printed = samplesMs.map((ms) => ms.toFixed(1)).join(', ');
  console.log(`cold start samples (ms): ${printed}`);
  console.log(
    `cold start median ${medianMs.toFixed(1)} ms over ${samplesMs.length} runs (budget ${budgetMs} ms)`,
  );
  return medianMs < budgetMs;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const reportPath = process.argv[2];
  if (!reportPath) {
    console.error('usage: cold-start-report.mjs <hyperfine.json>');
    process.exit(2);
  }
  const ok = reportStartup(JSON.parse(fs.readFileSync(reportPath, 'utf8')));
  if (!ok) process.exit(1);
  console.log('COLD_START_OK');
}

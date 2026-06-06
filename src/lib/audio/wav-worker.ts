/** Off-thread resample + WAV encode (decode stays on main thread). */

const TARGET_SAMPLE_RATE = 16000;

function resampleLinear(
  data: Float32Array,
  fromRate: number,
  toRate: number,
): Float32Array {
  if (fromRate === toRate) return data;
  const ratio = fromRate / toRate;
  const outLen = Math.max(1, Math.round(data.length / ratio));
  const out = new Float32Array(outLen);
  for (let i = 0; i < outLen; i++) {
    const srcIdx = i * ratio;
    const idx = Math.floor(srcIdx);
    const frac = srcIdx - idx;
    const a = data[idx] ?? 0;
    const b = data[Math.min(idx + 1, data.length - 1)] ?? 0;
    out[i] = a + (b - a) * frac;
  }
  return out;
}

function encodeMonoWav(samples: Float32Array, sampleRate: number): Uint8Array {
  const numSamples = samples.length;
  const bytesPerSample = 2;
  const blockAlign = bytesPerSample;
  const byteRate = sampleRate * blockAlign;
  const dataSize = numSamples * bytesPerSample;
  const headerSize = 44;
  const wavBuffer = new ArrayBuffer(headerSize + dataSize);
  const view = new DataView(wavBuffer);

  const writeString = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) {
      view.setUint8(offset + i, value.charCodeAt(i));
    }
  };

  writeString(0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeString(8, "WAVE");
  writeString(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, 16, true);
  writeString(36, "data");
  view.setUint32(40, dataSize, true);

  let offset = headerSize;
  for (let i = 0; i < numSamples; i++) {
    const clamped = Math.max(-1, Math.min(1, samples[i]));
    const pcm = clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff;
    view.setInt16(offset, pcm, true);
    offset += bytesPerSample;
  }

  return new Uint8Array(wavBuffer);
}

export type WavWorkerRequest = {
  samples: Float32Array;
  sampleRate: number;
};

export type WavWorkerResponse = {
  wav: Uint8Array;
};

self.onmessage = (event: MessageEvent<WavWorkerRequest>) => {
  const { samples, sampleRate } = event.data;
  const resampled = resampleLinear(samples, sampleRate, TARGET_SAMPLE_RATE);
  const wav = encodeMonoWav(resampled, TARGET_SAMPLE_RATE);
  const response: WavWorkerResponse = { wav };
  self.postMessage(response, { transfer: [wav.buffer] });
};

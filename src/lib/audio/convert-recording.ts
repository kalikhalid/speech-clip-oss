import WavWorker from "./wav-worker?worker";
import type { WavWorkerResponse } from "./wav-worker";

let worker: Worker | null = null;

function getWorker(): Worker {
  if (!worker) {
    worker = new WavWorker();
  }
  return worker;
}

function mixToMono(audioBuffer: AudioBuffer): Float32Array {
  const { length, numberOfChannels } = audioBuffer;
  if (numberOfChannels === 1) {
    return audioBuffer.getChannelData(0).slice();
  }
  const mono = new Float32Array(length);
  for (let ch = 0; ch < numberOfChannels; ch++) {
    const channel = audioBuffer.getChannelData(ch);
    for (let i = 0; i < length; i++) {
      mono[i] += channel[i];
    }
  }
  const scale = 1 / numberOfChannels;
  for (let i = 0; i < length; i++) {
    mono[i] *= scale;
  }
  return mono;
}

function convertInWorker(
  samples: Float32Array,
  sampleRate: number,
  format: "pcm_f32",
): Promise<Float32Array>;
function convertInWorker(
  samples: Float32Array,
  sampleRate: number,
  format: "wav",
): Promise<Uint8Array>;
function convertInWorker(
  samples: Float32Array,
  sampleRate: number,
  format: "pcm_f32" | "wav",
): Promise<Float32Array | Uint8Array> {
  return new Promise((resolve, reject) => {
    const w = getWorker();
    const onMessage = (event: MessageEvent<WavWorkerResponse>) => {
      w.removeEventListener("message", onMessage);
      w.removeEventListener("error", onError);
      if (event.data.format === "pcm_f32") {
        resolve(event.data.samples);
      } else {
        resolve(event.data.wav);
      }
    };
    const onError = (err: ErrorEvent) => {
      w.removeEventListener("message", onMessage);
      w.removeEventListener("error", onError);
      reject(err.error ?? new Error(String(err.message)));
    };
    w.addEventListener("message", onMessage);
    w.addEventListener("error", onError);
    w.postMessage({ samples, sampleRate, format }, [samples.buffer]);
  });
}

async function decodeRecordingToMono(
  audioBlob: Blob,
  existingContext?: AudioContext | null,
): Promise<{ samples: Float32Array; sampleRate: number }> {
  const encodedBuffer = await audioBlob.arrayBuffer();
  let decoderContext: AudioContext | null = null;
  let shouldCloseContext = false;
  try {
    if (existingContext && existingContext.state !== "closed") {
      decoderContext = existingContext;
    } else {
      decoderContext = new AudioContext();
      shouldCloseContext = true;
    }

    const decodedBuffer = await decoderContext.decodeAudioData(encodedBuffer);
    const samples = mixToMono(decodedBuffer);
    return { samples, sampleRate: decodedBuffer.sampleRate };
  } catch (error) {
    throw new Error(`Failed to decode recording: ${String(error)}`);
  } finally {
    if (decoderContext && shouldCloseContext) {
      try {
        await decoderContext.close();
      } catch {
        /* ignore */
      }
    }
  }
}

/** Decode on main thread; resample to 16 kHz mono f32 in a Web Worker. */
export async function convertRecordingToPcm16k(
  audioBlob: Blob,
  existingContext?: AudioContext | null,
): Promise<Uint8Array> {
  const { samples, sampleRate } = await decodeRecordingToMono(
    audioBlob,
    existingContext,
  );
  const pcm = await convertInWorker(samples, sampleRate, "pcm_f32");
  return new Uint8Array(pcm.buffer, pcm.byteOffset, pcm.byteLength);
}

/** Decode on main thread; resample + WAV encode in a Web Worker. */
export async function convertRecordingToWav(
  audioBlob: Blob,
  existingContext?: AudioContext | null,
): Promise<Uint8Array> {
  try {
    const { samples, sampleRate } = await decodeRecordingToMono(
      audioBlob,
      existingContext,
    );
    return convertInWorker(samples, sampleRate, "wav");
  } catch (error) {
    throw new Error(`Failed to convert recording to WAV: ${String(error)}`);
  }
}

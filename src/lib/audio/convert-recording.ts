import WavWorker from "./wav-worker?worker";

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

function encodeInWorker(
  samples: Float32Array,
  sampleRate: number,
): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const w = getWorker();
    const onMessage = (event: MessageEvent<{ wav: Uint8Array }>) => {
      w.removeEventListener("message", onMessage);
      w.removeEventListener("error", onError);
      resolve(event.data.wav);
    };
    const onError = (err: ErrorEvent) => {
      w.removeEventListener("message", onMessage);
      w.removeEventListener("error", onError);
      reject(err.error ?? new Error(String(err.message)));
    };
    w.addEventListener("message", onMessage);
    w.addEventListener("error", onError);
    w.postMessage({ samples, sampleRate }, [samples.buffer]);
  });
}

/** Decode on main thread; resample + WAV encode in a Web Worker. */
export async function convertRecordingToWav(
  audioBlob: Blob,
  existingContext?: AudioContext | null,
): Promise<Uint8Array> {
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

    const decodedBuffer = await decoderContext.decodeAudioData(
      encodedBuffer.slice(0),
    );
    const samples = mixToMono(decodedBuffer);
    return encodeInWorker(samples, decodedBuffer.sampleRate);
  } catch (error) {
    throw new Error(`Failed to convert recording to WAV: ${String(error)}`);
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

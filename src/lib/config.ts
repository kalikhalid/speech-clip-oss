// Overlay configuration - edit these values to customize
export const config = {
  // Idle state (small pill)
  idle: {
    width: 40,
    height: 10,
  },
  // Hover state (balanced size)
  hover: {
    width: 80,
    height: 20,
  },
  // Recording state (expanded with buttons)
  recording: {
    width: 110,
    height: 24,
  },
  // Position from bottom of screen
  bottomMargin: 20,
  // Visualizer
  barCount: 12,
  barWidth: 2,
  barMinHeight: 3,
  barMaxHeight: 18,
  barGap: 2,
  // Normalized speech-band level (0–1) required to animate bars
  voiceActivityThreshold: 0.07,
};

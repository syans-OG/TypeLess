import React, { useEffect, useState } from 'react';

export type SpideyState = 'idle' | 'listening' | 'transcribing' | 'done' | 'error';

interface MilesSpideyPetProps {
  state: SpideyState;
  audioRms?: number; // 0.0 to 1.0
  size?: number;
}

export const MilesSpideyPet: React.FC<MilesSpideyPetProps> = ({
  state = 'idle',
  audioRms = 0,
  size = 110,
}) => {
  const [blink, setBlink] = useState(false);
  const [wink, setWink] = useState(false);
  const [transcribePhase, setTranscribePhase] = useState<0 | 1>(0);

  // Natural idle/listening blinking loop
  useEffect(() => {
    let timeoutId: number;
    const scheduleBlink = () => {
      const delay = Math.random() * 3000 + 2000;
      timeoutId = window.setTimeout(() => {
        setBlink(true);
        window.setTimeout(() => {
          setBlink(false);
          scheduleBlink();
        }, 150);
      }, delay);
    };

    scheduleBlink();
    return () => clearTimeout(timeoutId);
  }, []);

  // Transcribing phase animation switcher (relaxed breathing searching rhythm)
  useEffect(() => {
    if (state !== 'transcribing') return;
    const interval = setInterval(() => {
      setTranscribePhase((prev) => (prev === 0 ? 1 : 0));
    }, 540);
    return () => clearInterval(interval);
  }, [state]);

  // Trigger celebratory wink on success
  useEffect(() => {
    if (state === 'done') {
      setWink(true);
      const timer = setTimeout(() => setWink(false), 750);
      return () => clearTimeout(timer);
    }
  }, [state]);

  // Audio sensitivity amplification (smooth knee)
  const amplifiedRms = Math.min(Math.max((audioRms - 0.015) * 4.5, 0), 1.0);

  // Reactive scaling and subtle head tilt
  const reactiveScale = state === 'listening' 
    ? 1 + amplifiedRms * 0.1 
    : state === 'transcribing' 
    ? 1.02 
    : 1;

  const headTilt = state === 'transcribing' 
    ? (transcribePhase === 0 ? -2.5 : 2.5)
    : state === 'listening' 
    ? (amplifiedRms > 0.25 ? 1.2 : 0) 
    : 0;

  // Luminous mask glow
  const glowOpacity = state === 'listening' 
    ? 0.4 + amplifiedRms * 0.45 
    : state === 'transcribing' 
    ? 0.85 
    : 0.2;

  // Eye lens morphing transforms based on avatar animation states
  let leftLensTransform = 'scale(1)';
  let rightLensTransform = 'scale(1)';

  if (blink) {
    leftLensTransform = 'scaleY(0.04)';
    rightLensTransform = 'scaleY(0.04)';
  } else if (wink) {
    // Happy celebrate / wink
    leftLensTransform = 'scale(1.14) translateY(-1.5px)';
    rightLensTransform = 'scaleY(0.08) rotate(12deg) translateY(5px)';
  } else if (state === 'listening') {
    // Listening / Gentle voice pulse
    const sx = 1 + amplifiedRms * 0.18;
    const sy = 1 + amplifiedRms * 0.26;
    const shiftY = -amplifiedRms * 2.2;
    leftLensTransform = `scale(${sx}, ${sy}) translateY(${shiftY}px) rotate(${-amplifiedRms * 2.5}deg)`;
    rightLensTransform = `scale(${sx}, ${sy}) translateY(${shiftY}px) rotate(${amplifiedRms * 2.5}deg)`;
  } else if (state === 'transcribing') {
    // Thinking / Searching Spider-Sense focus (Calm & Focused)
    if (transcribePhase === 0) {
      leftLensTransform = 'scaleY(0.55) scaleX(0.94) rotate(-7deg) translateY(-3px) translateX(1.5px)';
      rightLensTransform = 'scaleY(0.62) scaleX(0.96) rotate(5deg) translateY(-2px) translateX(1.5px)';
    } else {
      leftLensTransform = 'scaleY(0.62) scaleX(0.96) rotate(-5deg) translateY(-2px) translateX(-1.5px)';
      rightLensTransform = 'scaleY(0.55) scaleX(0.94) rotate(7deg) translateY(-3px) translateX(-1.5px)';
    }
  } else if (state === 'error') {
    leftLensTransform = 'scaleY(0.52) rotate(10deg)';
    rightLensTransform = 'scaleY(0.82) rotate(-6deg)';
  }

  const showSpiderSense = state === 'transcribing';

  return (
    <div
      style={{
        width: size,
        height: size,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative',
        transform: `scale(${reactiveScale}) rotate(${headTilt}deg)`,
        transition: 'transform 0.08s cubic-bezier(0.16, 1, 0.3, 1)',
        filter: `drop-shadow(0 0 ${10 * glowOpacity}px rgba(255, 42, 85, ${0.8 * glowOpacity})) drop-shadow(0 4px 12px rgba(0, 0, 0, 0.6))`,
      }}
    >
      <svg
        viewBox="0 0 240 260"
        width={size * 1.12}
        height={(size * 260 * 1.12) / 240}
        style={{ overflow: 'visible' }}
      >
        <defs>
          {/* Stealth Matte Black Mask Background */}
          <radialGradient id="spideyHeadGrad" cx="50%" cy="40%" r="60%">
            <stop offset="0%" stopColor="#1E202B" />
            <stop offset="55%" stopColor="#101117" />
            <stop offset="100%" stopColor="#050608" />
          </radialGradient>

          {/* Miles Morales Crimson Red Bezel Gradient */}
          <linearGradient id="milesCrimsonBezel" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stopColor="#FF2A55" />
            <stop offset="40%" stopColor="#E6103C" />
            <stop offset="100%" stopColor="#BA0526" />
          </linearGradient>

          {/* Luminous Inner Lens Gradient */}
          <radialGradient id="milesLensGlow" cx="50%" cy="45%" r="55%">
            <stop offset="0%" stopColor="#FFFFFF" />
            <stop offset="70%" stopColor="#F0F6FC" />
            <stop offset="100%" stopColor="#D9E6F2" />
          </radialGradient>
        </defs>

        {/* --- SPIDER-SENSE SQUIGGLY WAVES / LIGHTNING (Shows in Transcribing Phase) --- */}
        {showSpiderSense && (
          <g className="animate-spider-sense" style={{ transformOrigin: '120px 130px' }}>
            {/* Top Center Waves */}
            <path
              d="M 115 16 L 122 3 L 126 12 L 133 0"
              stroke="#2563EB"
              strokeWidth="5.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 115 16 L 122 3 L 126 12 L 133 0"
              stroke="#FF2A55"
              strokeWidth="3.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Top-Left Waves */}
            <path
              d="M 68 28 L 52 14 L 62 8 L 44 -2"
              stroke="#2563EB"
              strokeWidth="5.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 68 28 L 52 14 L 62 8 L 44 -2"
              stroke="#FF2A55"
              strokeWidth="3.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Top-Right Waves */}
            <path
              d="M 172 28 L 188 14 L 178 8 L 196 -2"
              stroke="#2563EB"
              strokeWidth="5.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 172 28 L 188 14 L 178 8 L 196 -2"
              stroke="#FF2A55"
              strokeWidth="3.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Far Left Waves */}
            <path
              d="M 28 85 L 8 82 L 18 94 L 0 92"
              stroke="#2563EB"
              strokeWidth="5.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 28 85 L 8 82 L 18 94 L 0 92"
              stroke="#FF2A55"
              strokeWidth="3.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Far Right Waves */}
            <path
              d="M 212 85 L 232 82 L 222 94 L 240 92"
              stroke="#2563EB"
              strokeWidth="5.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 212 85 L 232 82 L 222 94 L 240 92"
              stroke="#FF2A55"
              strokeWidth="3.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Mid-Left Squiggle */}
            <path
              d="M 32 145 L 14 154 L 22 164 L 6 172"
              stroke="#2563EB"
              strokeWidth="4.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 32 145 L 14 154 L 22 164 L 6 172"
              stroke="#FF2A55"
              strokeWidth="2.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />

            {/* Mid-Right Squiggle */}
            <path
              d="M 208 145 L 226 154 L 218 164 L 234 172"
              stroke="#2563EB"
              strokeWidth="4.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
            <path
              d="M 208 145 L 226 154 L 218 164 L 234 172"
              stroke="#FF2A55"
              strokeWidth="2.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
          </g>
        )}

        {/* --- MAIN HEAD & LENSES (Centered in 240x260 canvas) --- */}
        <g transform="translate(20, 15)">
          {/* --- 1. HEAD BASE SILHOUETTE --- */}
          <path
            d="M100 10 
               C165 10, 192 55, 192 110 
               C192 165, 155 218, 100 232 
               C45 218, 8 165, 8 110 
               C8 55, 35 10, 100 10 Z"
            fill="url(#spideyHeadGrad)"
            stroke="#FF1E46"
            strokeWidth="2.8"
            strokeOpacity="0.85"
          />

          {/* --- 2. AUTHENTIC SPIDER-VERSE WEB PATTERN --- */}
          <g stroke="#FF1E46" strokeWidth="1.2" strokeOpacity="0.32" fill="none">
            {/* Center Spine */}
            <path d="M100 12 L100 230" />
            
            {/* Radial Web Rays */}
            <path d="M100 115 L25 45" />
            <path d="M100 115 L175 45" />
            <path d="M100 115 L10 105" />
            <path d="M100 115 L190 105" />
            <path d="M100 115 L35 185" />
            <path d="M100 115 L165 185" />

            {/* Concentric Web Arcs */}
            <path d="M50 42 Q100 64 150 42" />
            <path d="M28 85 Q100 112 172 85" />
            <path d="M42 165 Q100 190 158 165" />
            <path d="M65 198 Q100 216 135 198" />
          </g>

          {/* --- 3. LEFT EYE LENS (Scaled & Balanced) --- */}
          <g transform="translate(8, 8) scale(0.85)">
            <g
              style={{
                transformOrigin: '56px 115px',
                transform: leftLensTransform,
                transition: 'transform 0.12s cubic-bezier(0.16, 1, 0.3, 1)',
              }}
            >
              {/* Outer Red Bezel */}
              <path
                d="M 92 118 
                   C 92 100, 88 64, 46 36 
                   C 36 30, 20 40, 16 54 
                   C 10 75, 8 116, 32 148 
                   C 52 174, 82 160, 90 134 
                   C 92 128, 92 122, 92 118 Z"
                fill="url(#milesCrimsonBezel)"
                stroke="#99001A"
                strokeWidth="1.8"
              />

              {/* Thin Inner Violet Accent Rim */}
              <path
                d="M 88 118 
                   C 88 102, 84 70, 48 44 
                   C 38 38, 26 47, 23 58 
                   C 18 76, 16 112, 38 142 
                   C 54 164, 78 152, 86 130 
                   C 88 125, 88 121, 88 118 Z"
                fill="none"
                stroke="#9333EA"
                strokeWidth="1.4"
                strokeOpacity="0.75"
              />

              {/* Inner Luminous White Lens */}
              <path
                d="M 85 118 
                   C 85 104, 81 74, 50 50 
                   C 42 45, 30 52, 28 62 
                   C 22 78, 22 110, 42 138 
                   C 56 156, 76 146, 83 128 
                   C 85 124, 85 120, 85 118 Z"
                fill="url(#milesLensGlow)"
              />

              {/* Specular White Highlight Glint */}
              <path
                d="M 38 72 C 48 60, 66 54, 74 52"
                stroke="#FFFFFF"
                strokeWidth="2.4"
                strokeLinecap="round"
                strokeOpacity="0.95"
                fill="none"
              />
            </g>
          </g>

          {/* --- 4. RIGHT EYE LENS (Scaled & Balanced) --- */}
          <g transform="translate(18, 8) scale(0.85)">
            <g
              style={{
                transformOrigin: '144px 115px',
                transform: rightLensTransform,
                transition: 'transform 0.12s cubic-bezier(0.16, 1, 0.3, 1)',
              }}
            >
              {/* Outer Red Bezel */}
              <path
                d="M 108 118 
                   C 108 100, 112 64, 154 36 
                   C 164 30, 180 40, 184 54 
                   C 190 75, 192 116, 168 148 
                   C 148 174, 118 160, 110 134 
                   C 108 128, 108 122, 108 118 Z"
                fill="url(#milesCrimsonBezel)"
                stroke="#99001A"
                strokeWidth="1.8"
              />

              {/* Thin Inner Violet Accent Rim */}
              <path
                d="M 112 118 
                   C 112 102, 116 70, 152 44 
                   C 162 38, 174 47, 177 58 
                   C 182 76, 184 112, 162 142 
                   C 146 164, 122 152, 114 130 
                   C 112 125, 112 121, 112 118 Z"
                fill="none"
                stroke="#9333EA"
                strokeWidth="1.4"
                strokeOpacity="0.75"
              />

              {/* Inner Luminous White Lens */}
              <path
                d="M 115 118 
               C 115 104, 119 74, 150 50 
               C 158 45, 170 52, 172 62 
               C 178 78, 178 110, 158 138 
               C 144 156, 124 146, 117 128 
               C 115 124, 115 120, 115 118 Z"
                fill="url(#milesLensGlow)"
              />

              {/* Specular White Highlight Glint */}
              <path
                d="M 162 72 C 152 60, 134 54, 126 52"
                stroke="#FFFFFF"
                strokeWidth="2.4"
                strokeLinecap="round"
                strokeOpacity="0.95"
                fill="none"
              />
            </g>
          </g>
        </g>
      </svg>
    </div>
  );
};

export default MilesSpideyPet;

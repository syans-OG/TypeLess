import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { MilesSpideyPet, SpideyState } from './MilesSpideyPet';

export const PetOverlay: React.FC = () => {
  const [state, setState] = useState<SpideyState>('idle');
  const [audioRms, setAudioRms] = useState<number>(0);
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    let exitTimer: ReturnType<typeof setTimeout> | null = null;

    // Listen for voice typing state changes from Rust backend
    const unlistenStatePromise = listen<{ status: string; text?: string }>(
      'pill-state',
      (event) => {
        const status = event.payload.status as SpideyState;
        setState(status);

        if (status === 'listening') {
          if (exitTimer) clearTimeout(exitTimer);
          setIsExiting(false);
        } else if (status === 'done') {
          // Trigger smooth exit animation after the celebratory wink
          if (exitTimer) clearTimeout(exitTimer);
          exitTimer = setTimeout(() => {
            setIsExiting(true);
          }, 820);
        }
      }
    );

    // Listen for real-time RMS audio level stream
    const unlistenRmsPromise = listen<number>('audio-rms', (event) => {
      setAudioRms(event.payload);
    });

    return () => {
      if (exitTimer) clearTimeout(exitTimer);
      unlistenStatePromise.then((unlisten) => unlisten());
      unlistenRmsPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Equalizer bar heights calculated dynamically
  const getBarHeight = (multiplier: number, minHeight = 4) => {
    const calculated = Math.min(Math.max(audioRms * 32 * multiplier, minHeight), 28);
    return `${calculated}px`;
  };

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-end',
        paddingRight: '28px',
        paddingLeft: '28px',
        paddingTop: '20px',
        paddingBottom: '20px',
        boxSizing: 'border-box',
        background: 'transparent',
        overflow: 'visible',
        userSelect: 'none',
      }}
    >
      <div
        className={isExiting ? 'animate-spidey-exit' : 'animate-spidey-enter'}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '14px',
          background: 'transparent',
          overflow: 'visible',
          transformOrigin: 'center center',
        }}
      >
        {/* Soundwave Mini Bubble (Appears ONLY while listening/recording) */}
        {state === 'listening' && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              padding: '8px 14px',
              background: 'rgba(15, 17, 23, 0.88)',
              backdropFilter: 'blur(20px)',
              WebkitBackdropFilter: 'blur(20px)',
              border: '1px solid rgba(239, 68, 68, 0.35)',
              borderRadius: '9999px',
              boxShadow: '0 8px 24px -4px rgba(0, 0, 0, 0.7), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
              animation: 'fadeIn 0.18s ease-out',
              height: '42px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '3px', height: '24px' }}>
              <span
                style={{
                  width: '3px',
                  height: getBarHeight(0.6, 5),
                  borderRadius: '999px',
                  background: '#EF4444',
                  transition: 'height 0.08s ease-out',
                }}
              />
              <span
                style={{
                  width: '3px',
                  height: getBarHeight(1.1, 7),
                  borderRadius: '999px',
                  background: '#FF3355',
                  transition: 'height 0.08s ease-out',
                }}
              />
              <span
                style={{
                  width: '3.5px',
                  height: getBarHeight(1.6, 10),
                  borderRadius: '999px',
                  background: '#FFFFFF',
                  transition: 'height 0.08s ease-out',
                }}
              />
              <span
                style={{
                  width: '3px',
                  height: getBarHeight(1.2, 7),
                  borderRadius: '999px',
                  background: '#FF3355',
                  transition: 'height 0.08s ease-out',
                }}
              />
              <span
                style={{
                  width: '3px',
                  height: getBarHeight(0.7, 5),
                  borderRadius: '999px',
                  background: '#EF4444',
                  transition: 'height 0.08s ease-out',
                }}
              />
            </div>
          </div>
        )}

        {/* Miles Morales Spidey Mascot */}
        <MilesSpideyPet state={state} audioRms={audioRms} size={88} />
      </div>
    </div>
  );
};

export default PetOverlay;

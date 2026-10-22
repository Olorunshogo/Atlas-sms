/*
 * Ported from the reference source's React lib/motion/variants.ts. The ORIGINAL file is a
 * framer-motion Variants/Transition catalogue - framer-motion is
 * React-only, so there is no direct Vue equivalent of its `Variants` object
 * shape (initial/animate/exit keyed on AnimatePresence). What carries over
 * is the underlying design tokens: durations, the easing curve, and the
 * keyframe values themselves, which any Vue animation approach (the GSAP
 * engine already in ../../motion/presets.ts, or plain CSS transitions) can
 * consume directly.
 *
 * Only ./presets.ts's dialog enter/exit timeline is actually wired up today.
 * Everything else below (cart controls, sticky bar, collapse, select menu,
 * loading dot, stagger, video player) is tokens only - a GSAP tween or CSS
 * transition using these values still needs to be written per component
 * before use.
 */

// === Timing tokens

export const motionDuration = {
  /** Micro-feedback: chips, hovers, colour changes. */
  fast: 0.15,
  /** Default for opacity and small offsets. */
  base: 0.25,
  /** Deliberate, for larger surfaces entering. */
  slow: 0.4,
} as const;

/** Standard ease for non-spring motion, as a cubic-bezier tuple (CSS `cubic-bezier()` or GSAP `CustomEase`). */
export const motionEase = [0.22, 1, 0.36, 1] as const;

// === Spring presets (GSAP equivalents - see gsap's "elastic"/physics2D plugins for a literal spring, or approximate with duration + overshoot-ease)

export interface SpringToken {
  damping: number;
  stiffness: number;
}

/** Panels and sheets. Settles firmly with no visible wobble. */
export const springPanel: SpringToken = { damping: 28, stiffness: 320 };

/** Small elements that should feel lively: badges, checkmarks, icons. */
export const springPop: SpringToken = { damping: 24, stiffness: 300 };

// === Keyframe tokens
//
// Each value pair is the from/to state a Vue component animates between
// (via GSAP tween or CSS transition), equivalent to the source's
// initial/animate/exit Variants objects.

export const fadeKeyframes = { from: { opacity: 0 }, to: { opacity: 1 } };

/** Fade plus a short rise. The default entrance for content blocks. */
export const fadeUpKeyframes = {
  from: { opacity: 0, y: 16 },
  to: { opacity: 1, y: 0 },
};

/** Fade plus a short drop. For banners and alerts arriving from above. */
export const fadeDownKeyframes = {
  from: { opacity: 0, y: -8 },
  to: { opacity: 1, y: 0 },
};

// === Dialog

export const backdropKeyframes = fadeKeyframes;

/** Bottom sheet on mobile, centered panel on desktop. Pair with springPanel. */
export const dialogPanelKeyframes = {
  from: { y: 48, opacity: 0 },
  to: { y: 0, opacity: 1 },
};

/** Full-screen overlays such as a document viewer. */
export const fullScreenOverlayKeyframes = fadeUpKeyframes;

/** Success confirmation. Pair with springPop. */
export const successPanelKeyframes = {
  from: { opacity: 0, scale: 0.96 },
  to: { opacity: 1, scale: 1 },
};

/**
 * How long a dialog stays open showing its success panel before closing.
 * Closing instantly reads as if nothing happened, and anything under ~2s is
 * too quick to actually read the confirmation.
 */
export const DIALOG_SUCCESS_CLOSE_DELAY_MS = 2500;

// === Swapped content

export const swappedContentKeyframes = fadeKeyframes;
export const swappedContentDurationS = motionDuration.fast;

// === Cart controls

/** Swapping a product card's "Add to cart" button for its quantity stepper. */
export const cartControlKeyframes = {
  from: { y: 24, opacity: 0 },
  to: { y: 0, opacity: 1 },
  exit: { y: -24, opacity: 0 },
};

/** A bar docked to the bottom of the viewport that appears in response to state, such as the cart summary bar. */
export const stickyBarKeyframes = {
  from: { y: 80, opacity: 0 },
  to: { y: 0, opacity: 1 },
};

// === Inline disclosure

/** A panel expanding in place: an accordion row, a drill-down level. Animate height to "auto". */
export const collapseKeyframes = {
  from: { opacity: 0, height: 0 },
  to: { opacity: 1, height: 'auto' as const },
};

// === Select menus

export const selectMenuKeyframes = {
  from: { opacity: 0, scaleY: 0.92, y: -4 },
  to: { opacity: 1, scaleY: 1, y: 0 },
};

/** The same motion for a menu that flipped above its trigger - offset mirrored so it still travels away from the trigger. */
export const selectMenuUpKeyframes = {
  from: { opacity: 0, scaleY: 0.92, y: 4 },
  to: { opacity: 1, scaleY: 1, y: 0 },
};

/** Fast on purpose: a menu that takes its time feels broken, not premium. */
export const selectMenuDurationS = motionDuration.fast;

// === Loading ellipsis

/** One dot's pulse in a loading ellipsis. */
export const loadingDotKeyframes = {
  opacity: [0.4, 1, 0.4],
  scale: [0.85, 1, 0.85],
};
export const loadingDotDurationS = 1.2;

// === Staggered lists

export const staggerItemKeyframes = {
  from: { opacity: 0, y: 16 },
  to: { opacity: 1, y: 0 },
};

/** Per-item delay (seconds) for a staggered grid or list entrance. */
export function staggerDelayS(index: number, step = 0.06): number {
  return index * step;
}

// === Video player

export const videoPosterKeyframes = {
  from: { opacity: 0, scale: 1.03 },
  to: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.985 },
};
export const videoScrimKeyframes = fadeKeyframes;
export const videoPlayButtonKeyframes = {
  from: { opacity: 0, scale: 0.7 },
  to: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.82 },
};

/** Soft ring that breathes behind the play button while idle/paused. */
export const videoPlayRingKeyframes = {
  scale: [1, 1.18, 1],
  opacity: [0.35, 0.12, 0.35],
};
export const videoPlayRingDurationS = 2.4;

/** After the video ends, hold the last frame briefly before returning to the poster. */
export const VIDEO_END_RESET_DELAY_MS = 1600;

export const videoAtmosphereKeyframes = {
  from: { opacity: 0, scale: 1.08 },
  to: { opacity: 1, scale: 1.12 },
  exit: { opacity: 0, scale: 1.08 },
};
export const videoStageKeyframes = {
  from: { opacity: 0, scale: 0.94 },
  to: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.97 },
};

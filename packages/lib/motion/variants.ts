import type { Transition, Variants } from 'framer-motion';

/*
 * The single source of truth for motion in this codebase.
 *
 * Never declare an inline `initial`/`animate`/`exit` object or a one-off `Variants` in a component or page.
 * Import from here.
 * Uniform motion is what makes an interface feel deliberate rather than assembled, and a shared file is the only way that survives more than one contributor.
 *
 * If a genuinely new motion is needed, add it here and name it after the job it does, not the values it uses.
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

/** Standard ease for non-spring motion. Avoids the sluggish feel of ease-in. */
export const motionEase = [0.22, 1, 0.36, 1] as const;

// === Spring presets

/** Panels and sheets. Settles firmly with no visible wobble. */
export const springPanel: Transition = {
  type: 'spring',
  damping: 28,
  stiffness: 320,
};

/** Small elements that should feel lively: badges, checkmarks, icons. */
export const springPop: Transition = {
  type: 'spring',
  damping: 24,
  stiffness: 300,
};

// === Plain transitions

export const transitionFast: Transition = {
  duration: motionDuration.fast,
  ease: motionEase,
};

export const transitionBase: Transition = {
  duration: motionDuration.base,
  ease: motionEase,
};

/** Large surfaces that should feel scenic rather than snappy: a rotating hero image. */
export const transitionSlow: Transition = {
  duration: motionDuration.slow,
  ease: motionEase,
};

// === Fades

export const fadeVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
};

/** Fade plus a short rise. The default entrance for content blocks. */
export const fadeUpVariants: Variants = {
  initial: { opacity: 0, y: 16 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: 16 },
};

/** Fade plus a short drop. For banners and alerts arriving from above. */
export const fadeDownVariants: Variants = {
  initial: { opacity: 0, y: -8 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -8 },
};

// === Dialog

export const backdropVariants: Variants = fadeVariants;

/** Bottom sheet on mobile, centered panel on desktop. Pair with springPanel. */
export const dialogPanelVariants: Variants = {
  initial: { y: 48, opacity: 0 },
  animate: { y: 0, opacity: 1 },
  exit: { y: 48, opacity: 0 },
};

/** Full-screen overlays such as a document viewer. */
export const fullScreenOverlayVariants: Variants = fadeUpVariants;

/** Success confirmation. Pair with springPop. */
export const successPanelVariants: Variants = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.96 },
};

/*
 * How long a dialog stays open showing its success panel before closing.
 * Closing instantly reads as if nothing happened, and anything under ~2s is too quick to actually read the confirmation.
 */
export const DIALOG_SUCCESS_CLOSE_DELAY_MS = 2500;

// === Swapped content

/*
 * Crossfading content that replaces itself in place: a paginated list, a tab panel, a step in a wizard.
 * A plain opacity crossfade on purpose - a transform here reads as noise because the surrounding chrome has not moved.
 * Use with AnimatePresence mode="wait" and a `key` on the changing content.
 */
export const swappedContentVariants: Variants = fadeVariants;

export const swappedContentTransition: Transition = transitionFast;

// === Cart controls

/*
 * Swapping a product card's "Add to cart" button for its quantity stepper.
 *
 * Unlike swappedContentVariants this DOES translate: the two controls occupy the same slot and the vertical slide is what communicates that one replaced the other, rather than the card having re-rendered.
 * Pair with springPanel and AnimatePresence mode="wait", and give the slot a fixed height so the grid does not reflow on every add.
 */
export const cartControlVariants: Variants = {
  initial: { y: 24, opacity: 0 },
  animate: { y: 0, opacity: 1 },
  exit: { y: -24, opacity: 0 },
};

/*
 * A bar docked to the bottom of the viewport that appears in response to state, such as the cart summary bar.
 * Rises into place rather than fading, so it reads as arriving from off-screen.
 */
export const stickyBarVariants: Variants = {
  initial: { y: 80, opacity: 0 },
  animate: { y: 0, opacity: 1 },
  exit: { y: 80, opacity: 0 },
};

// === Inline disclosure

/*
 * A panel expanding in place: an accordion row, a drill-down level appearing under the one above it.
 * Animating `height` to "auto" is what keeps the surrounding layout honest rather than overlapping it.
 */
export const collapseVariants: Variants = {
  initial: { opacity: 0, height: 0 },
  animate: { opacity: 1, height: 'auto' },
  exit: { opacity: 0, height: 0 },
};

// === Select menus

/*
 * A select's option list opening beneath its trigger.
 *
 * Scales from the top edge rather than fading in place, so the menu reads as unfolding out of the trigger it belongs to.
 * Keep the origin at the top in the component (`origin-top`) or the scale looks like it grew from the middle.
 */
export const selectMenuVariants: Variants = {
  initial: { opacity: 0, scaleY: 0.92, y: -4 },
  animate: { opacity: 1, scaleY: 1, y: 0 },
  exit: { opacity: 0, scaleY: 0.92, y: -4 },
};

/*
 * The same motion for a menu that flipped above its trigger.
 *
 * The offset is mirrored so the menu still travels away from the trigger, not through it.
 * Pair with `origin-bottom`, or it scales from the wrong edge and the flip reads as a glitch.
 */
export const selectMenuUpVariants: Variants = {
  initial: { opacity: 0, scaleY: 0.92, y: 4 },
  animate: { opacity: 1, scaleY: 1, y: 0 },
  exit: { opacity: 0, scaleY: 0.92, y: 4 },
};

/* Fast on purpose: a menu that takes its time feels broken, not premium. */
export const selectMenuTransition: Transition = {
  duration: motionDuration.fast,
  ease: motionEase,
};

// === Loading ellipsis

/** One dot's pulse in a loading ellipsis - pair with staggerDelay's `step` for the travelling-pulse effect. */
export const loadingDotVariants: Variants = {
  animate: { opacity: [0.4, 1, 0.4], scale: [0.85, 1, 0.85] },
};

export const loadingDotTransition: Transition = {
  duration: 1.2,
  repeat: Infinity,
  ease: 'easeInOut',
};

// === Staggered lists

export const staggerItemVariants: Variants = {
  initial: { opacity: 0, y: 16 },
  animate: { opacity: 1, y: 0 },
};

/** Per-item delay for a staggered grid or list entrance. */
export function staggerDelay(index: number, step = 0.06): Transition {
  return {
    duration: motionDuration.slow,
    ease: motionEase,
    delay: index * step,
  };
}

// === Video player

/*
 * Click-to-play poster and mid-playback pause overlay.
 * Soft scale so the still feels like it settles into place rather than popping.
 */
export const videoPosterVariants: Variants = {
  initial: { opacity: 0, scale: 1.03 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.985 },
};

export const videoPosterTransition: Transition = {
  duration: motionDuration.slow,
  ease: motionEase,
};

/** Dimmer over the frame when paused or on the poster. Pair with transitionBase. */
export const videoScrimVariants: Variants = fadeVariants;

/*
 * The circular play affordance.
 * Pair with springPop so it lands with a short, deliberate bounce.
 */
export const videoPlayButtonVariants: Variants = {
  initial: { opacity: 0, scale: 0.7 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.82 },
};

/** Soft ring that breathes behind the play button while idle/paused. */
export const videoPlayRingVariants: Variants = {
  animate: {
    scale: [1, 1.18, 1],
    opacity: [0.35, 0.12, 0.35],
  },
};

export const videoPlayRingTransition: Transition = {
  duration: 2.4,
  repeat: Infinity,
  ease: 'easeInOut',
};

/** After the video ends, hold the last frame briefly before returning to the poster. */
export const VIDEO_END_RESET_DELAY_MS = 1600;

/*
 * Blurred poster that fills the cinema letterbox gutters behind an inset portrait stage.
 * Slightly overscaled so the blur does not hard-clip at the edges.
 */
export const videoAtmosphereVariants: Variants = {
  initial: { opacity: 0, scale: 1.08 },
  animate: { opacity: 1, scale: 1.12 },
  exit: { opacity: 0, scale: 1.08 },
};

export const videoAtmosphereTransition: Transition = {
  duration: motionDuration.slow,
  ease: motionEase,
};

/*
 * The sharp inset stage (poster or video) sitting in the centre of the wide frame.
 * Pair with springPop on enter so it settles into the glass surround.
 */
export const videoStageVariants: Variants = {
  initial: { opacity: 0, scale: 0.94 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.97 },
};

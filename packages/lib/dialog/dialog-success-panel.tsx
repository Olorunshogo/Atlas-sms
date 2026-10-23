import { CheckCircle2 } from 'lucide-react';
import { motion } from 'framer-motion';
import { successPanelVariants, springPop } from '../motion/variants';

/*
 * Replaces a dialog's form when a submission succeeds.
 *
 * Use this instead of closing the dialog the instant a request resolves - closing instantly reads as if nothing happened.
 * The glue component should hold the dialog open before calling closeDialog, using DIALOG_SUCCESS_CLOSE_DELAY_MS so every dialog in the app waits the same amount of time.
 */

export interface DialogSuccessPanelProps {
  title: string;
  description?: string;
}

export function DialogSuccessPanel({
  title,
  description,
}: DialogSuccessPanelProps) {
  return (
    <motion.div
      variants={successPanelVariants}
      initial="initial"
      animate="animate"
      transition={springPop}
      className="flex flex-col items-center gap-3 py-6 text-center"
      role="status"
      aria-live="polite"
    >
      <CheckCircle2 className="text-good-green h-12 w-12" />
      <h2 className="font-syne text-heading text-lg font-bold">{title}</h2>
      {description && <p className="text-body text-sm">{description}</p>}
    </motion.div>
  );
}

import { cva, type VariantProps } from 'class-variance-authority';

export const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        default:
          'bg-primary text-primary-foreground shadow hover:bg-primary/90 cursor-pointer duration-300 ease-in-out',
        destructive:
          'bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 cursor-pointer duration-300 ease-in-out',
        outline:
          'border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground cursor-pointer duration-300 ease-in-out',
        secondary:
          'bg-teal text-ink-inverse shadow-sm hover:bg-teal-hover cursor-pointer duration-300 ease-in-out',
        ghost:
          'bg-transparent text-ink-primary hover:bg-surface-sunken cursor-pointer duration-300 ease-in-out',
        link: 'text-primary underline-offset-4 hover:underline cursor-pointer duration-300 ease-in-out',
      },
      size: {
        default: 'h-9 px-4 py-2',
        sm: 'h-8 rounded-md px-3 text-xs',
        lg: 'h-10 rounded-md px-8',
        icon: 'h-12 w-12',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
);

export type ButtonVariants = VariantProps<typeof buttonVariants>;

export type HiredRole = 'employee' | 'customer_care' | 'tech';
export declare function announceRoleChanged(role: string | undefined): void;
export declare function onRoleChanged(
  handler: (role: string | undefined) => void,
): () => void;

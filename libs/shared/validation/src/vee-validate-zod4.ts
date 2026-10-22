import * as z from 'zod';

/*
 * @vee-validate/zod's toTypedSchema only supports zod ^3.24.0 (confirmed against its latest published release, 4.15.1); this workspace pins zod 4.3.6, and Zod v4 schema instances don't satisfy toTypedSchema's v3-shaped internal type checks.
 * This is a direct re-implementation of vee-validate's TypedSchema contract against Zod v4's safeParseAsync, so every form keeps the same useForm({ validationSchema: toZod4TypedSchema(schema) }) call shape it already used with toTypedSchema.
 * Delete this file and switch call sites back to @vee-validate/zod's toTypedSchema the day it ships real Zod v4 support.
 */

interface TypedSchemaError {
  path?: string;
  errors: string[];
}

interface TypedSchema<TInput = unknown, TOutput = TInput> {
  __type: 'VVTypedSchema';
  parse: (
    values: TInput,
  ) => Promise<{ value?: TOutput; errors: TypedSchemaError[] }>;
}

export function toZod4TypedSchema<TSchema extends z.ZodType>(
  schema: TSchema,
): TypedSchema<z.input<TSchema>, z.output<TSchema>> {
  return {
    __type: 'VVTypedSchema',
    async parse(values: z.input<TSchema>) {
      const result = await schema.safeParseAsync(values);
      if (result.success) {
        return { value: result.data, errors: [] };
      }

      const errorsByPath = new Map<string, string[]>();
      for (const issue of result.error.issues) {
        const path = issue.path.join('.');
        const existing = errorsByPath.get(path) ?? [];
        existing.push(issue.message);
        errorsByPath.set(path, existing);
      }

      const errors: TypedSchemaError[] = Array.from(errorsByPath.entries()).map(
        ([path, messages]) => ({
          path: path || undefined,
          errors: messages,
        }),
      );

      return { errors };
    },
  };
}

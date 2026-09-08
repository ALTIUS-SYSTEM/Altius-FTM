import { z } from 'zod';
import { DateOnlySchema, IdSchema, NonnegativeIntegerSchema } from './primitives';

export const CheckStatusSchema = z.enum(['good', 'damaged', 'present', 'missing', 'na']);

export const CheckCategorySchema = z.enum(['equipment', 'inspection']);

export const CheckItemSchema = z.object({
  name: z.string().min(1).max(120),
  category: CheckCategorySchema,
  status: CheckStatusSchema,
  note: z.string().max(500),
}).strict();

export const VehicleConditionSchema = z.enum(['good', 'not_good']);

export const VehicleCheckSchema = z.object({
  id: IdSchema,
  tenantId: IdSchema,
  hubId: IdSchema,
  driverId: IdSchema,
  day: DateOnlySchema,
  driverName: z.string().min(1).max(120),
  licensePlate: z.string().min(1).max(40),
  vehicleType: z.string().min(1).max(80),
  kmStart: NonnegativeIntegerSchema,
  kmEnd: NonnegativeIntegerSchema,
  condition: VehicleConditionSchema,
  items: z.array(CheckItemSchema).max(100).readonly(),
  notes: z.string().max(2000),
  serviceDate: DateOnlySchema.nullable(),
  kirDate: DateOnlySchema.nullable(),
  stnkDate: DateOnlySchema.nullable(),
  createdAt: z.string().datetime(),
}).strict().superRefine((value, context) => {
  if (value.kmEnd < value.kmStart) {
    context.addIssue({ code: 'custom', message: 'kmEnd cannot be less than kmStart' });
  }
  if (new Set(value.items.map(item => `${item.category}:${item.name}`)).size !== value.items.length) {
    context.addIssue({ code: 'custom', message: 'Duplicate checklist item names within a category' });
  }
});

export type CheckStatus = z.infer<typeof CheckStatusSchema>;
export type CheckCategory = z.infer<typeof CheckCategorySchema>;
export type CheckItem = z.infer<typeof CheckItemSchema>;
export type VehicleCondition = z.infer<typeof VehicleConditionSchema>;
export type VehicleCheck = z.infer<typeof VehicleCheckSchema>;

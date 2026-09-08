import { z } from 'zod';
import { DateOnlySchema, DeviceTimeSchema, IdSchema, MoneySchema, NonnegativeIntegerSchema, RevisionSchema, TimeZoneSchema, UtcTimestampSchema } from './primitives';

export const ExpenseCategorySchema = z.enum(['fuel', 'toll', 'parking', 'meal', 'maintenance', 'other']);
export const ExpenseSchema = z.object({ id: IdSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, revision: RevisionSchema, time: DeviceTimeSchema, category: ExpenseCategorySchema, money: MoneySchema, description: z.string().min(1).max(1000), receiptId: IdSchema.nullable() }).strict().readonly();
export const LhsStatusSchema = z.enum(['draft', 'submitted', 'revision_requested', 'approved']);
export const LhsReviewRevisionSchema = z.object({ revision: RevisionSchema, action: z.enum(['submit', 'request_revision', 'resubmit', 'approve']), actorId: IdSchema, atUtc: UtcTimestampSchema, note: z.string().min(1).max(2000).nullable() }).strict().superRefine((value, context) => {
  if (value.action === 'request_revision' && value.note === null) context.addIssue({ code: 'custom', message: 'Revision request requires a note' });
}).readonly();
const lhsReportFields = z.object({
  id: IdSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, day: DateOnlySchema, timeZone: TimeZoneSchema, revision: RevisionSchema, status: LhsStatusSchema,
  visitedTaskIds: z.array(IdSchema).max(1000).readonly(), completedStopIds: z.array(IdSchema).max(2000).readonly(), expenses: z.array(ExpenseSchema).max(200).readonly(), totals: z.array(MoneySchema).max(16).readonly(),
  routeVersions: z.array(z.object({ routeId: IdSchema, version: RevisionSchema }).strict().readonly()).max(100).readonly(),
  reviews: z.array(LhsReviewRevisionSchema).max(100).readonly()
}).strict();
export const LhsReportSchema = lhsReportFields.superRefine((value, context) => {
  const issue = (message: string) => context.addIssue({ code: 'custom', message });
  if (new Set(value.visitedTaskIds).size !== value.visitedTaskIds.length || new Set(value.completedStopIds).size !== value.completedStopIds.length || new Set(value.expenses.map(expense => expense.id)).size !== value.expenses.length) issue('Duplicate report entries');
  if (value.expenses.some(expense => expense.tenantId !== value.tenantId || expense.hubId !== value.hubId || expense.driverId !== value.driverId)) issue('Expense scope mismatch');
  const totals = new Map<string, number>();
  for (const expense of value.expenses) {
    const sum = (totals.get(expense.money.currency) ?? 0) + expense.money.amountMinor;
    if (!Number.isSafeInteger(sum)) issue('Money total exceeds safe integer');
    totals.set(expense.money.currency, sum);
  }
  if (new Set(value.totals.map(total => total.currency)).size !== value.totals.length || value.totals.length !== totals.size || value.totals.some(total => totals.get(total.currency) !== total.amountMinor)) issue('Incorrect currency totals');
  let state: z.infer<typeof LhsStatusSchema> = 'draft';
  let revision = -1;
  let time = -Infinity;
  for (const review of value.reviews) {
    if (review.revision <= revision || review.revision > value.revision || Date.parse(review.atUtc) < time) issue('Review revisions must be ordered');
    const next: z.infer<typeof LhsStatusSchema> | null = state === 'draft' && review.action === 'submit' ? 'submitted' : state === 'submitted' && review.action === 'request_revision' ? 'revision_requested' : state === 'revision_requested' && review.action === 'resubmit' ? 'submitted' : state === 'submitted' && review.action === 'approve' ? 'approved' : null;
    if (next === null) issue('Invalid review transition'); else state = next;
    revision = review.revision;
    time = Date.parse(review.atUtc);
  }
  if (state !== value.status) issue('Review history does not match report status');
}).readonly();
export const LhsSummarySchema = z.object({ tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, day: DateOnlySchema, timeZone: TimeZoneSchema, visitedTaskIds: z.array(IdSchema).readonly(), completedStopIds: z.array(IdSchema).readonly(), totals: z.array(MoneySchema).readonly(), eventCount: NonnegativeIntegerSchema }).strict().readonly();
export type ExpenseCategory = z.infer<typeof ExpenseCategorySchema>;
export type Expense = z.infer<typeof ExpenseSchema>;
export type LhsStatus = z.infer<typeof LhsStatusSchema>;
export type LhsReviewRevision = z.infer<typeof LhsReviewRevisionSchema>;
export type LhsReport = z.infer<typeof LhsReportSchema>;
export type LhsSummary = z.infer<typeof LhsSummarySchema>;

/**
 * Request-side report: `status`, `revision` and `reviews` are server-owned.
 *
 * Accepting them from the submitting client let a driver post their own daily
 * report as `approved`, naming any actor id as the approver — the money path
 * with the review step removed. Approval transitions belong on their own
 * endpoint, carrying only `{reportId, action, note}` with the actor taken from
 * the token.
 */
export const LhsReportRequestSchema = lhsReportFields.omit({
  status: true,
  revision: true,
  reviews: true,
});
export type LhsReportRequest = z.infer<typeof LhsReportRequestSchema>;

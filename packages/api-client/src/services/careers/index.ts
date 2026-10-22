import { apiFetch, apiMutate } from '../../apiFetch';
import { publicPost, publicRequest } from '../../transport/public-request';

export type HrEmploymentType = 'full_time' | 'part_time' | 'contract' | 'agent';

export interface HrJob {
  id: number;
  title: string;
  department: string;
  location: string;
  description: string;
  requirements: string;
  employment_type: HrEmploymentType;
  experience_min_years?: number | null;
  experience_max_years?: number | null;
  tech_stack?: string[];
  salary_min_kobo?: number | null;
  salary_max_kobo?: number | null;
  status: 'open' | 'closed' | 'filled';
  created_at?: string;
}

export interface OpenJobsFacets {
  employment_type: Record<string, number>;
}

export interface OpenJobsResponse {
  jobs: HrJob[];
  facets: OpenJobsFacets;
}

export interface ListOpenJobsParams {
  q?: string;
  location?: string;
  employment_type?: string;
}

export interface HrApplication {
  id: number;
  job_id: number;
  user_id: number;
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  cover_letter?: string | null;
  cv_url?: string | null;
  status: string;
  created_at?: string;
}

export interface HrInterview {
  id: number;
  application_id: number;
  scheduled_at: string;
  mode?: string | null;
  location?: string | null;
  status?: string;
  feedback?: string | null;
}

export interface HrOffer {
  id: number;
  application_id: number;
  position: string;
  salary_kobo: number;
  start_date: string;
  expires_at: string;
  status: string;
  benefits?: string | null;
  letter_snapshot?: string | null;
}

export interface CreateJobPayload {
  title: string;
  department: string;
  location: string;
  description: string;
  requirements: string;
  employment_type?: HrEmploymentType;
  experience_min_years?: number | null;
  experience_max_years?: number | null;
  tech_stack?: string[];
  salary_min_kobo?: number | null;
  salary_max_kobo?: number | null;
}

/*
 * Email is not sent: the backend takes it from the signed-in account.
 * phone should already be normalised to the local 0XXXXXXXXXX form.
 */
export interface ApplyJobFields {
  first_name: string;
  last_name: string;
  phone: string;
  cover_letter: string;
  cv: File;
}

export interface RegisterApplicantPayload {
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  password: string;
  confirm_password: string;
}

export function listOpenJobs(
  params: ListOpenJobsParams = {},
): Promise<OpenJobsResponse> {
  const search = new URLSearchParams();
  if (params.q) search.set('q', params.q);
  if (params.location) search.set('location', params.location);
  if (params.employment_type)
    search.set('employment_type', params.employment_type);
  const qs = search.toString();
  return publicRequest<OpenJobsResponse>(`/careers/jobs${qs ? `?${qs}` : ''}`);
}

export function getOpenJob(id: number): Promise<HrJob> {
  return publicRequest<HrJob>(`/careers/jobs/${id}`);
}

export function applyToJob(
  jobId: number,
  fields: ApplyJobFields,
): Promise<{ application: HrApplication; job: HrJob }> {
  const form = new FormData();
  form.append('first_name', fields.first_name);
  form.append('last_name', fields.last_name);
  form.append('phone', fields.phone);
  form.append('cover_letter', fields.cover_letter);
  form.append('cv', fields.cv);
  return apiMutate<{ application: HrApplication; job: HrJob }>(
    `/careers/jobs/${jobId}/applications`,
    { method: 'POST', body: form },
  );
}

export function registerApplicant(
  payload: RegisterApplicantPayload,
): Promise<unknown> {
  return publicPost('/careers/register', payload);
}

export function listJobsForHr(): Promise<HrJob[]> {
  return apiFetch<HrJob[]>('/careers/admin/jobs');
}

export function createJob(payload: CreateJobPayload): Promise<HrJob> {
  return apiMutate<HrJob>('/careers/admin/jobs', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateJob(
  id: number,
  payload: Partial<CreateJobPayload> & { status?: HrJob['status'] },
): Promise<HrJob> {
  return apiMutate<HrJob>(`/careers/admin/jobs/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function listApplications(jobId?: number): Promise<HrApplication[]> {
  const q = jobId !== undefined ? `?job_id=${jobId}` : '';
  return apiFetch<HrApplication[]>(`/careers/admin/applications${q}`);
}

export function getApplicationCvUrl(
  applicationId: number,
): Promise<{ url: string }> {
  return apiFetch<{ url: string }>(
    `/careers/admin/applications/${applicationId}/cv`,
  );
}

export function screenApplication(
  id: number,
  payload: {
    status: 'screening' | 'passed' | 'rejected';
    notes?: string;
    rejection_feedback?: string;
  },
): Promise<HrApplication> {
  return apiMutate<HrApplication>(`/careers/admin/applications/${id}/screen`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function scheduleInterview(
  applicationId: number,
  payload: {
    scheduled_at: string;
    location: string;
    interviewer_user_id?: number | null;
  },
): Promise<HrInterview> {
  return apiMutate<HrInterview>(
    `/careers/admin/applications/${applicationId}/interviews`,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
}

export function scheduleMyInterview(
  applicationId: number,
  payload: {
    scheduled_at: string;
    location: string;
  },
): Promise<HrInterview> {
  return apiMutate<HrInterview>(
    `/careers/me/applications/${applicationId}/interviews`,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
}

export function createOffer(
  applicationId: number,
  payload: {
    position: string;
    engagement?: 'team' | 'intern' | 'volunteer';
    salary_kobo: number;
    start_date: string;
    expires_at: string;
    benefits?: string;
    employment_type?: 'full_time' | 'part_time' | 'contract' | 'agent';
    department?: string;
    manager_user_id?: number | null;
  },
): Promise<HrOffer> {
  return apiMutate<HrOffer>(
    `/careers/admin/applications/${applicationId}/offers`,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
}

export function listMyApplications(): Promise<
  { application: HrApplication; job: HrJob; interview: HrInterview | null }[]
> {
  return apiFetch('/careers/me/applications');
}

export function listMyOffers(): Promise<
  { offer: HrOffer; application: HrApplication; job: HrJob }[]
> {
  return apiFetch('/careers/me/offers');
}

// `role` is the role the account now holds: "employee", "customer_care", or "tech".
export function acceptOffer(offerId: number): Promise<{ role?: string }> {
  return apiMutate(`/careers/offers/${offerId}/accept`, { method: 'POST' });
}

export function rejectOffer(offerId: number): Promise<unknown> {
  return apiMutate(`/careers/offers/${offerId}/reject`, { method: 'POST' });
}

export function hrAcceptOffer(offerId: number): Promise<unknown> {
  return apiMutate(`/careers/admin/offers/${offerId}/accept`, {
    method: 'POST',
  });
}

export interface HrSignRequest {
  id: number;
  kind: string;
  status: string;
  title: string;
  sign_url?: string | null;
  signed_document_url?: string | null;
  signed_at?: string | null;
  metadata?: string | null;
  subject_user_id: number;
  /** The document to preview before signing - the subject's contract, or the policy's PDF. Null when there's nothing to preview. */
  preview_url?: string | null;
}

export function signMyContract(): Promise<HrSignRequest> {
  return apiMutate('/careers/me/contract/sign', { method: 'POST' });
}

export function listMySignRequests(): Promise<HrSignRequest[]> {
  return apiFetch('/careers/sign-requests');
}

export interface HrEmployeeProfile {
  id: number;
  user_id: number;
  job_title: string;
  department: string;
  status: string;
  contract_url?: string | null;
  start_date?: string;
  emergency_contact_name?: string | null;
  emergency_contact_relationship?: string | null;
  emergency_contact_phone?: string | null;
  next_of_kin?: string | null;
  home_address?: string | null;
  date_of_birth?: string | null;
}

export interface HrAccountUser {
  id: number;
  first_name: string;
  last_name: string;
  email: string;
  phone?: string | null;
  role: string;
  avatar_url?: string | null;
  name_confirmed_at?: string | null;
  name_changed_at?: string | null;
  name_change_count?: number;
}

export interface HrMyEmployeeProfileResponse {
  employee: HrEmployeeProfile | null;
  user: HrAccountUser;
}

export function getMyEmployeeProfile(): Promise<HrMyEmployeeProfileResponse> {
  return apiFetch('/careers/employees/me');
}

export type SetupChecklistItemId =
  | 'photo'
  | 'name'
  | 'handbook'
  | 'emergency'
  | 'birthday';

export interface EmployeeWorkspaceOverview {
  user: HrAccountUser;
  employee: HrEmployeeProfile | null;
  setup: {
    items: { id: SetupChecklistItemId; done: boolean }[];
    completed: number;
    total: number;
    done: boolean;
    progress_percent: number;
  };
  weekly: {
    report_due: boolean;
    report_sent_this_week: boolean;
    policies_outstanding: number;
    reports_this_month: number;
    contract_signed: boolean;
    show_outreach: boolean;
  };
  report_weeks: { week: string; count: number }[];
  name_change: {
    can_change: boolean;
    next_change_at: string | null;
    changes_used: number;
  };
  /** null unless weekly.show_outreach is true. */
  outreach: OutreachLeadStats | null;
  stats: {
    policies_left: number;
    reports_this_month: number;
    contract_label: string;
    contract_trend: string;
  };
}

export interface OutreachLeadStats {
  leads_total: number;
  leads_called: number;
}

export function getEmployeeWorkspaceOverview(): Promise<EmployeeWorkspaceOverview> {
  return apiFetch('/careers/employees/me/workspace');
}

export function updateMyEmployeeName(input: {
  first_name: string;
  last_name: string;
}): Promise<HrMyEmployeeProfileResponse> {
  return apiMutate('/careers/employees/me/name', {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function updateMyEmployeeProfileFields(input: {
  phone?: string;
  home_address?: string | null;
  next_of_kin?: string | null;
  date_of_birth?: string | null;
}): Promise<HrMyEmployeeProfileResponse> {
  return apiMutate('/careers/employees/me', {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function updateMyEmergencyContact(input: {
  emergency_contact_name: string;
  emergency_contact_relationship: string;
  emergency_contact_phone: string;
}): Promise<HrMyEmployeeProfileResponse> {
  return apiMutate('/careers/employees/me/emergency-contact', {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function uploadMyEmployeeAvatar(
  file: File,
): Promise<HrMyEmployeeProfileResponse> {
  const form = new FormData();
  form.append('file', file);
  return apiMutate('/careers/employees/me/avatar', {
    method: 'POST',
    body: form,
  });
}

export function changeEmployeePassword(input: {
  old_password: string;
  new_password: string;
}): Promise<{ message: string }> {
  return apiMutate('/careers/employees/me/password', {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function getEmployeeProfile(
  employeeUserId: number,
): Promise<HrMyEmployeeProfileResponse> {
  return apiMutate('/careers/employees/profile', {
    method: 'POST',
    body: JSON.stringify({ employee_user_id: employeeUserId }),
  });
}

export interface HrPolicy {
  id: number;
  title: string;
  slug: string;
  body: string;
  document_url?: string | null;
  version: string;
  audience: 'all' | 'technical';
  restricted: boolean;
  effective_at: string;
}

export interface HrPolicyAcknowledgment {
  id: number;
  policy_id: number;
  user_id: number;
  acknowledged_at: string;
  policy_version: string;
}

export function listPolicies(): Promise<HrPolicy[]> {
  return apiFetch('/careers/policies');
}

export function listMyAcknowledgments(): Promise<HrPolicyAcknowledgment[]> {
  return apiFetch('/careers/policies/acknowledgments/me');
}

export function acknowledgePolicy(
  policyId: number,
): Promise<HrPolicyAcknowledgment> {
  return apiMutate(`/careers/policies/${policyId}/acknowledge`, {
    method: 'POST',
  });
}

export interface HrDirectoryRow {
  employee: HrEmployeeProfile;
  contract_signed_at?: string | null;
  // Agents are staff; they do not come through the jobs pipeline.
  staff_kind?: 'employee' | 'agent';
  agent_status?: string | null;
  user: {
    id: number;
    first_name: string;
    last_name: string;
    email: string;
    phone?: string | null;
    role: string;
    avatar_url?: string | null;
  };
}

export function listDirectory(opts?: {
  q?: string;
  department?: string;
  status?: string;
}): Promise<HrDirectoryRow[]> {
  const params = new URLSearchParams();
  if (opts?.q) params.set('q', opts.q);
  if (opts?.department) params.set('department', opts.department);
  if (opts?.status) params.set('status', opts.status);
  const qs = params.toString();
  return apiFetch(`/careers/directory${qs ? `?${qs}` : ''}`);
}

export function uploadEmployeeContract(
  userId: number,
  contract: File,
): Promise<HrEmployeeProfile> {
  const form = new FormData();
  form.append('contract', contract);
  return apiFetch(`/careers/employees/${userId}/docs`, {
    method: 'PATCH',
    body: form,
  });
}

export type OutreachActivityKind = 'call' | 'message';

export interface OutreachActivityLogSummary {
  id: number;
  done_by_name: string;
  done_by_role: string;
  created_at: string;
}

export interface OutreachActivityState {
  /** How many checkboxes to render; the server rejects a tick past this. */
  target: number;
  logs: OutreachActivityLogSummary[];
  /** Behind this week's target. Never true for a record with no phone. */
  pending: boolean;
}

export interface OutreachNoteSummary {
  note: string;
  picture_urls: string[];
  author_name: string;
  updated_at: string | null;
}

export interface OutreachActivityRecord {
  id: number;
  shop_name: string;
  owner_name: string | null;
  phone: string | null;
  additional_phones?: string[];
  lga: string | null;
  area: string | null;
  address: string | null;
  calls: OutreachActivityState;
  messages: OutreachActivityState;
  note: OutreachNoteSummary | null;
}

export interface OutreachActivityList {
  week_start: string;
  current_week_start: string;
  is_current_week: boolean;
  records: OutreachActivityRecord[];
}

export function listOutreachRecords<T = unknown>(): Promise<T[]> {
  return apiFetch('/careers/outreach/records');
}

/** Omit weekStart for the live current week; pass a Monday (YYYY-MM-DD) to look at a past week, read-only there. */
export function listOutreachActivity(
  weekStart?: string,
): Promise<OutreachActivityList> {
  return apiFetch(
    `/careers/outreach${weekStart ? `?week_start=${weekStart}` : ''}`,
  );
}

export function tickOutreachActivity(
  outreachId: number,
  activityType: OutreachActivityKind,
): Promise<OutreachActivityLogSummary> {
  return apiMutate(`/careers/outreach/${outreachId}/activity`, {
    method: 'POST',
    body: JSON.stringify({ activity_type: activityType }),
  });
}

export function untickOutreachActivity(
  logId: number,
): Promise<{ message: string }> {
  return apiMutate(`/careers/outreach/activity/${logId}`, {
    method: 'DELETE',
  });
}

export interface OutreachTargets {
  call_weekly_target: number;
  message_weekly_target: number;
}

export function getOutreachTargets(): Promise<OutreachTargets> {
  return apiFetch('/careers/outreach/targets');
}

export function updateOutreachTargets(
  input: OutreachTargets,
): Promise<OutreachTargets> {
  return apiMutate('/careers/admin/outreach/targets', {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function saveOutreachNote(
  outreachId: number,
  input: { note: string; pictures: string[] },
): Promise<OutreachNoteSummary> {
  return apiMutate(`/careers/outreach/${outreachId}/note`, {
    method: 'PUT',
    body: JSON.stringify(input),
  });
}

export interface WorkReport {
  id: number;
  period_type: string;
  period_start: string;
  period_end: string;
  tasks_completed: string;
  milestones: string | null;
  challenges: string | null;
  next_steps: string | null;
  status: string;
  reviewer_notes: string | null;
  created_at: string;
}

export function listMyWorkReports(): Promise<WorkReport[]> {
  return apiFetch('/careers/work-reports/me');
}

export function submitWorkReport(body: {
  period_type: 'daily' | 'weekly' | 'monthly';
  period_start: string;
  period_end: string;
  tasks_completed: string;
  challenges?: string;
  next_steps?: string;
  submit: boolean;
}): Promise<WorkReport> {
  return apiFetch('/careers/work-reports', {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

// === Expense reports

export type ExpenseCategory =
  | 'travel'
  | 'meals'
  | 'materials'
  | 'client_entertainment'
  | 'other';

export type ExpenseStatus =
  | 'pending'
  | 'manager_approved'
  | 'admin_approved'
  | 'rejected'
  | 'reimbursed';

export interface ExpenseReport {
  id: number;
  /* Already on the wire from listExpensesAdmin (a plain select() of the whole row) - just missing from this type until the admin review UI needed to show whose expense it is. */
  employee_user_id: number;
  description: string;
  amount_kobo: number;
  expense_date: string;
  category: ExpenseCategory;
  receipt_url: string | null;
  status: ExpenseStatus;
  reviewer_notes: string | null;
  reviewed_at: string | null;
  created_at: string;
}

export function listMyExpenses(): Promise<ExpenseReport[]> {
  return apiFetch('/careers/expenses/me');
}

export function submitExpenseReport(input: {
  description: string;
  amount_kobo: number;
  expense_date: string;
  category: ExpenseCategory;
  receipt?: File;
}): Promise<ExpenseReport> {
  const form = new FormData();
  form.append('description', input.description);
  form.append('amount_kobo', String(input.amount_kobo));
  form.append('expense_date', input.expense_date);
  form.append('category', input.category);
  if (input.receipt) form.append('receipt', input.receipt);
  return apiMutate('/careers/expenses', { method: 'POST', body: form });
}

// === Incident reports

export type IncidentCategory =
  | 'safety'
  | 'hr_issue'
  | 'conflict'
  | 'damage'
  | 'other';

export type IncidentStatus =
  | 'reported'
  | 'under_investigation'
  | 'resolved'
  | 'closed';

export interface IncidentReport {
  id: number;
  /* Already on the wire from listIncidents (a plain select() of the whole row) - just missing from this type until the admin review UI needed to show who reported it. */
  reporter_user_id: number;
  occurred_at: string;
  description: string;
  people_involved: string | null;
  impact: string | null;
  corrective_actions: string | null;
  category: IncidentCategory;
  critical: boolean;
  status: IncidentStatus;
  created_at: string;
}

export function listIncidentReports(): Promise<IncidentReport[]> {
  return apiFetch('/careers/incidents');
}

export function submitIncidentReport(input: {
  occurred_at: string;
  description: string;
  people_involved?: string;
  impact?: string;
  corrective_actions?: string;
  category: IncidentCategory;
  critical?: boolean;
}): Promise<IncidentReport> {
  return apiMutate('/careers/incidents', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

// === Project status reports

export type ProjectReportStatus = 'draft' | 'submitted' | 'approved';

export interface ProjectStatusReport {
  id: number;
  /* Already on the wire from listProjectReportsAdmin (a plain select() of the whole row) - just missing from this type until the admin review UI needed to show whose report it is. */
  employee_user_id: number;
  project_name: string;
  progress_pct: number;
  completed_items: string | null;
  blockers: string | null;
  next_steps: string | null;
  status: ProjectReportStatus;
  reviewed_at: string | null;
  created_at: string;
}

export function listMyProjectReports(): Promise<ProjectStatusReport[]> {
  return apiFetch('/careers/project-reports/me');
}

export function submitProjectReport(input: {
  project_name: string;
  progress_pct: number;
  completed_items?: string;
  blockers?: string;
  next_steps?: string;
  submit?: boolean;
}): Promise<ProjectStatusReport> {
  return apiMutate('/careers/project-reports', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

// === Performance reviews / PIP

export type PerformanceReviewType = 'annual' | 'probation';
export type PerformanceReviewStatus =
  | 'pending_self'
  | 'pending_manager'
  | 'completed';

export interface PerformanceReview {
  id: number;
  employee_user_id: number;
  manager_user_id: number | null;
  review_type: PerformanceReviewType;
  due_at: string;
  status: PerformanceReviewStatus;
  self_achievements: string | null;
  self_learnings: string | null;
  self_goals_met: string | null;
  self_improvements: string | null;
  self_next_goals: string | null;
  self_submitted_at: string | null;
  manager_goals_met: boolean | null;
  manager_rating: number | null;
  manager_feedback: string | null;
  manager_submitted_at: string | null;
  probation_decision: string | null;
  created_at: string;
}

export function listMyPerformanceReviews(): Promise<PerformanceReview[]> {
  return apiFetch('/careers/performance/reviews');
}

export function submitSelfAssessment(
  reviewId: number,
  input: {
    self_achievements: string;
    self_learnings?: string;
    self_goals_met?: string;
    self_improvements?: string;
    self_next_goals?: string;
  },
): Promise<PerformanceReview> {
  return apiMutate(`/careers/performance/reviews/${reviewId}/self`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export type PipStatus = 'active' | 'passed' | 'failed';

export interface PerformancePlan {
  id: number;
  employee_user_id: number;
  manager_user_id: number;
  goals: string;
  timelines: string;
  support: string | null;
  success_criteria: string;
  status: PipStatus;
  due_at: string;
  outcome_notes: string | null;
  closed_at: string | null;
  created_at: string;
}

export function listMyPips(): Promise<PerformancePlan[]> {
  return apiFetch('/careers/performance/pips');
}

// === Admin review (desk admin or super admin only - scoped server-side)

export function createPerformanceReview(input: {
  employee_user_id: number;
  review_type?: 'annual' | 'probation';
  due_at: string;
  manager_user_id?: number | null;
}): Promise<PerformanceReview> {
  return apiMutate('/careers/performance/reviews', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function submitManagerReview(
  reviewId: number,
  input: {
    manager_goals_met: boolean;
    manager_rating: number;
    manager_feedback?: string;
    probation_decision?: 'confirm' | 'terminate';
  },
): Promise<PerformanceReview> {
  return apiMutate(`/careers/performance/reviews/${reviewId}/manager`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function createPip(input: {
  employee_user_id: number;
  goals: string;
  timelines: string;
  support?: string;
  success_criteria: string;
  due_at: string;
}): Promise<PerformancePlan> {
  return apiMutate('/careers/performance/pips', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function closePip(
  pipId: number,
  input: { status: 'passed' | 'failed'; outcome_notes?: string },
): Promise<PerformancePlan> {
  return apiMutate(`/careers/performance/pips/${pipId}/close`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function listExpensesAdmin(): Promise<ExpenseReport[]> {
  return apiFetch('/careers/admin/expenses');
}

export function reviewExpense(
  expenseId: number,
  input: {
    status: 'manager_approved' | 'admin_approved' | 'rejected' | 'reimbursed';
    reviewer_notes?: string;
  },
): Promise<ExpenseReport> {
  return apiMutate(`/careers/admin/expenses/${expenseId}/review`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function updateIncident(
  incidentId: number,
  input: {
    status: IncidentStatus;
    assignee_user_id?: number | null;
    corrective_actions?: string;
  },
): Promise<IncidentReport> {
  return apiMutate(`/careers/admin/incidents/${incidentId}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function listProjectReportsAdmin(): Promise<ProjectStatusReport[]> {
  return apiFetch('/careers/admin/project-reports');
}

export function approveProjectReport(
  reportId: number,
): Promise<ProjectStatusReport> {
  return apiMutate(`/careers/admin/project-reports/${reportId}/approve`, {
    method: 'PATCH',
  });
}

// === Careers-wide analytics (hiring funnel, headcount, HR load)

export interface CareersAnalytics {
  jobs: { open: number; closed: number; filled: number; total: number };
  applications: {
    total: number;
    received: number;
    passed: number;
    rejected: number;
  };
  employees: {
    total: number;
    by_department: { department: string; count: number }[];
  };
  leave_pending: number;
  work_reports_submitted: number;
  avg_time_to_hire_days: number | null;
  conversion: { applied_to_passed: number; applied_to_hired: number };
}

export function getCareersAnalytics(): Promise<CareersAnalytics> {
  return apiFetch('/careers/admin/analytics');
}

export interface TechThroughput {
  department: string;
  period_start: string;
  active_employees: number;
  reports_submitted_this_period: number;
  reports_pending_review: number;
  average_progress_pct: number | null;
}

export function getTechThroughput(): Promise<TechThroughput> {
  return apiFetch('/careers/admin/analytics/tech-throughput');
}

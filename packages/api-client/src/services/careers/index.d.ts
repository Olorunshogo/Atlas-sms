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
export declare function listOpenJobs(
  params?: ListOpenJobsParams,
): Promise<OpenJobsResponse>;
export declare function getOpenJob(id: number): Promise<HrJob>;
export declare function applyToJob(
  jobId: number,
  fields: ApplyJobFields,
): Promise<{
  application: HrApplication;
  job: HrJob;
}>;
export declare function registerApplicant(
  payload: RegisterApplicantPayload,
): Promise<unknown>;
export declare function listJobsForHr(): Promise<HrJob[]>;
export declare function createJob(payload: CreateJobPayload): Promise<HrJob>;
export declare function updateJob(
  id: number,
  payload: Partial<CreateJobPayload> & {
    status?: HrJob['status'];
  },
): Promise<HrJob>;
export declare function listApplications(
  jobId?: number,
): Promise<HrApplication[]>;
export declare function getApplicationCvUrl(applicationId: number): Promise<{
  url: string;
}>;
export declare function screenApplication(
  id: number,
  payload: {
    status: 'screening' | 'passed' | 'rejected';
    notes?: string;
    rejection_feedback?: string;
  },
): Promise<HrApplication>;
export declare function scheduleInterview(
  applicationId: number,
  payload: {
    scheduled_at: string;
    location: string;
    interviewer_user_id?: number | null;
  },
): Promise<HrInterview>;
export declare function scheduleMyInterview(
  applicationId: number,
  payload: {
    scheduled_at: string;
    location: string;
  },
): Promise<HrInterview>;
export declare function createOffer(
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
): Promise<HrOffer>;
export declare function listMyApplications(): Promise<
  {
    application: HrApplication;
    job: HrJob;
    interview: HrInterview | null;
  }[]
>;
export declare function listMyOffers(): Promise<
  {
    offer: HrOffer;
    application: HrApplication;
    job: HrJob;
  }[]
>;
export declare function acceptOffer(offerId: number): Promise<{
  role?: string;
}>;
export declare function rejectOffer(offerId: number): Promise<unknown>;
export declare function hrAcceptOffer(offerId: number): Promise<unknown>;
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
export declare function signMyContract(): Promise<HrSignRequest>;
export declare function listMySignRequests(): Promise<HrSignRequest[]>;
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
export declare function getMyEmployeeProfile(): Promise<HrMyEmployeeProfileResponse>;
export type SetupChecklistItemId = 'photo' | 'name' | 'handbook' | 'emergency';
export interface EmployeeWorkspaceOverview {
  user: HrAccountUser;
  employee: HrEmployeeProfile | null;
  setup: {
    items: {
      id: SetupChecklistItemId;
      done: boolean;
    }[];
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
  report_weeks: {
    week: string;
    count: number;
  }[];
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
export declare function getEmployeeWorkspaceOverview(): Promise<EmployeeWorkspaceOverview>;
export declare function updateMyEmployeeName(input: {
  first_name: string;
  last_name: string;
}): Promise<HrMyEmployeeProfileResponse>;
export declare function updateMyEmployeeProfileFields(input: {
  phone?: string;
  home_address?: string | null;
  next_of_kin?: string | null;
}): Promise<HrMyEmployeeProfileResponse>;
export declare function updateMyEmergencyContact(input: {
  emergency_contact_name: string;
  emergency_contact_relationship: string;
  emergency_contact_phone: string;
}): Promise<HrMyEmployeeProfileResponse>;
export declare function uploadMyEmployeeAvatar(
  file: File,
): Promise<HrMyEmployeeProfileResponse>;
export declare function changeEmployeePassword(input: {
  old_password: string;
  new_password: string;
}): Promise<{
  message: string;
}>;
export declare function getEmployeeProfile(
  employeeUserId: number,
): Promise<HrMyEmployeeProfileResponse>;
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
export declare function listPolicies(): Promise<HrPolicy[]>;
export declare function listMyAcknowledgments(): Promise<
  HrPolicyAcknowledgment[]
>;
export declare function acknowledgePolicy(
  policyId: number,
): Promise<HrPolicyAcknowledgment>;
export interface HrDirectoryRow {
  employee: HrEmployeeProfile;
  contract_signed_at?: string | null;
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
export declare function listDirectory(opts?: {
  q?: string;
  department?: string;
  status?: string;
}): Promise<HrDirectoryRow[]>;
export declare function uploadEmployeeContract(
  userId: number,
  contract: File,
): Promise<HrEmployeeProfile>;
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
export declare function listOutreachRecords<T = unknown>(): Promise<T[]>;
/** Omit weekStart for the live current week; pass a Monday (YYYY-MM-DD) to look at a past week, read-only there. */
export declare function listOutreachActivity(
  weekStart?: string,
): Promise<OutreachActivityList>;
export declare function tickOutreachActivity(
  outreachId: number,
  activityType: OutreachActivityKind,
): Promise<OutreachActivityLogSummary>;
export declare function untickOutreachActivity(logId: number): Promise<{
  message: string;
}>;
export interface OutreachTargets {
  call_weekly_target: number;
  message_weekly_target: number;
}
export declare function getOutreachTargets(): Promise<OutreachTargets>;
export declare function updateOutreachTargets(
  input: OutreachTargets,
): Promise<OutreachTargets>;
export declare function saveOutreachNote(
  outreachId: number,
  input: {
    note: string;
    pictures: string[];
  },
): Promise<OutreachNoteSummary>;
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
export declare function listMyWorkReports(): Promise<WorkReport[]>;
export declare function submitWorkReport(body: {
  period_type: 'daily' | 'weekly' | 'monthly';
  period_start: string;
  period_end: string;
  tasks_completed: string;
  challenges?: string;
  next_steps?: string;
  submit: boolean;
}): Promise<WorkReport>;
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
export declare function listMyExpenses(): Promise<ExpenseReport[]>;
export declare function submitExpenseReport(input: {
  description: string;
  amount_kobo: number;
  expense_date: string;
  category: ExpenseCategory;
  receipt?: File;
}): Promise<ExpenseReport>;
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
export declare function listIncidentReports(): Promise<IncidentReport[]>;
export declare function submitIncidentReport(input: {
  occurred_at: string;
  description: string;
  people_involved?: string;
  impact?: string;
  corrective_actions?: string;
  category: IncidentCategory;
  critical?: boolean;
}): Promise<IncidentReport>;
export type ProjectReportStatus = 'draft' | 'submitted' | 'approved';
export interface ProjectStatusReport {
  id: number;
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
export declare function listMyProjectReports(): Promise<ProjectStatusReport[]>;
export declare function submitProjectReport(input: {
  project_name: string;
  progress_pct: number;
  completed_items?: string;
  blockers?: string;
  next_steps?: string;
  submit?: boolean;
}): Promise<ProjectStatusReport>;
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
export declare function listMyPerformanceReviews(): Promise<
  PerformanceReview[]
>;
export declare function submitSelfAssessment(
  reviewId: number,
  input: {
    self_achievements: string;
    self_learnings?: string;
    self_goals_met?: string;
    self_improvements?: string;
    self_next_goals?: string;
  },
): Promise<PerformanceReview>;
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
export declare function listMyPips(): Promise<PerformancePlan[]>;
export declare function createPerformanceReview(input: {
  employee_user_id: number;
  review_type?: 'annual' | 'probation';
  due_at: string;
  manager_user_id?: number | null;
}): Promise<PerformanceReview>;
export declare function submitManagerReview(
  reviewId: number,
  input: {
    manager_goals_met: boolean;
    manager_rating: number;
    manager_feedback?: string;
    probation_decision?: 'confirm' | 'terminate';
  },
): Promise<PerformanceReview>;
export declare function createPip(input: {
  employee_user_id: number;
  goals: string;
  timelines: string;
  support?: string;
  success_criteria: string;
  due_at: string;
}): Promise<PerformancePlan>;
export declare function closePip(
  pipId: number,
  input: {
    status: 'passed' | 'failed';
    outcome_notes?: string;
  },
): Promise<PerformancePlan>;
export declare function listExpensesAdmin(): Promise<ExpenseReport[]>;
export declare function reviewExpense(
  expenseId: number,
  input: {
    status: 'manager_approved' | 'admin_approved' | 'rejected' | 'reimbursed';
    reviewer_notes?: string;
  },
): Promise<ExpenseReport>;
export declare function updateIncident(
  incidentId: number,
  input: {
    status: IncidentStatus;
    assignee_user_id?: number | null;
    corrective_actions?: string;
  },
): Promise<IncidentReport>;
export declare function listProjectReportsAdmin(): Promise<
  ProjectStatusReport[]
>;
export declare function approveProjectReport(
  reportId: number,
): Promise<ProjectStatusReport>;
export interface CareersAnalytics {
  jobs: {
    open: number;
    closed: number;
    filled: number;
    total: number;
  };
  applications: {
    total: number;
    received: number;
    passed: number;
    rejected: number;
  };
  employees: {
    total: number;
    by_department: {
      department: string;
      count: number;
    }[];
  };
  leave_pending: number;
  work_reports_submitted: number;
  avg_time_to_hire_days: number | null;
  conversion: {
    applied_to_passed: number;
    applied_to_hired: number;
  };
}
export declare function getCareersAnalytics(): Promise<CareersAnalytics>;
export interface TechThroughput {
  department: string;
  period_start: string;
  active_employees: number;
  reports_submitted_this_period: number;
  reports_pending_review: number;
  average_progress_pct: number | null;
}
export declare function getTechThroughput(): Promise<TechThroughput>;

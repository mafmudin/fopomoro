export interface FoTask {
  id: string;
  task_id: string;
  title: string;
  is_completed: boolean;
  created_at: string;
  completed_at: string | null;
  pomodoro_count: number;
}

export interface FoSession {
  date: string;
  focus_sessions_count: number;
  total_minutes_studied: number;
  tasks_completed_count: number;
}

export interface PomodoroConfig {
  focus_minutes: number;
  short_break_minutes: number;
  long_break_minutes: number;
}

export interface WindowSettings {
  opacity: number;
}

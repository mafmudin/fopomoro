-- FoPoMoro P0 — Tahap 0: isolasi data per-user.
-- Jalankan di Supabase Dashboard → SQL Editor (atau `supabase db push`).
--
-- ⚠️  DESTRUKTIF: skrip ini DROP & recreate `tasks` + `pomodoro_sessions`.
--     Ini disengaja — data lama dibuat di era anon (tanpa user_id) dan tidak
--     terpakai. Pastikan tak ada data penting sebelum menjalankan.
--
-- Setelah migrasi ini, semua operasi data HARUS pakai JWT user (Bearer
-- access_token), bukan anon key. Anon key hanya untuk endpoint /auth/v1/*.

begin;

drop table if exists public.pomodoro_sessions cascade;
drop table if exists public.tasks cascade;

-- ── tasks ────────────────────────────────────────────────────────────────
create table public.tasks (
  id             uuid        primary key default gen_random_uuid(),
  user_id        uuid        not null default auth.uid()
                              references auth.users (id) on delete cascade,
  task_number    int         not null,          -- diisi otomatis per-user (trigger)
  title          text        not null,
  is_completed   boolean     not null default false,
  created_at     timestamptz not null default now(),
  completed_at   timestamptz,
  pomodoro_count int         not null default 0,
  unique (user_id, task_number)                  -- FO-NN unik per user
);

-- ── pomodoro_sessions ────────────────────────────────────────────────────
create table public.pomodoro_sessions (
  id               uuid        primary key default gen_random_uuid(),
  user_id          uuid        not null default auth.uid()
                                references auth.users (id) on delete cascade,
  task_id          uuid        references public.tasks (id) on delete set null,
  duration_minutes int         not null,
  was_focused      boolean     not null,
  created_at       timestamptz not null default now()
);

-- ── Penomoran FO-NN per-user ───────────────────────────────────────────────
-- INSERT dari app tidak mengirim task_number; trigger mengisinya = max milik
-- user tsb + 1. Default kolom (auth.uid()) sudah terisi sebelum BEFORE trigger
-- berjalan, jadi NEW.user_id valid di sini.
create or replace function public.assign_task_number()
returns trigger
language plpgsql
as $$
begin
  if new.task_number is null then
    select coalesce(max(task_number), 0) + 1
      into new.task_number
      from public.tasks
     where user_id = new.user_id;
  end if;
  return new;
end;
$$;

create trigger trg_assign_task_number
  before insert on public.tasks
  for each row execute function public.assign_task_number();

-- ── Row Level Security ─────────────────────────────────────────────────────
alter table public.tasks             enable row level security;
alter table public.pomodoro_sessions enable row level security;

-- Hanya pemilik (auth.uid()) yang bisa lihat/ubah barisnya. Tak ada policy
-- untuk role `anon` → anon key tidak bisa baca/tulis sama sekali.
create policy "tasks_owner_all" on public.tasks
  for all to authenticated
  using (user_id = auth.uid())
  with check (user_id = auth.uid());

create policy "sessions_owner_all" on public.pomodoro_sessions
  for all to authenticated
  using (user_id = auth.uid())
  with check (user_id = auth.uid());

commit;

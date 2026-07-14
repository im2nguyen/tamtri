import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import { persistedStorage } from "@/lib/persisted-storage";

export type OnboardingPhase = "welcome" | "gate" | "starter" | "complete";

export const ONBOARDING_VERSION = 1;

interface OnboardingState {
  onboarding_version: number;
  onboarding_phase: OnboardingPhase;
  it_request_sent_at: string | null;
  sample_worked_at: string | null;
  my_file_worked_at: string | null;
  example_dismissed: boolean;
  permission_coached_at: string | null;
  sample_run_conversation_id: string | null;
  setPhase: (phase: OnboardingPhase) => void;
  markItRequestSent: () => void;
  markSampleWorked: () => void;
  markMyFileWorked: () => void;
  markPermissionCoached: () => void;
  dismissExample: () => void;
  setSampleRunConversationId: (id: string | null) => void;
  completeOnboarding: () => void;
}

export const useOnboardingStore = create<OnboardingState>()(
  persist(
    (set) => ({
      onboarding_version: ONBOARDING_VERSION,
      onboarding_phase: "welcome",
      it_request_sent_at: null,
      sample_worked_at: null,
      my_file_worked_at: null,
      example_dismissed: false,
      permission_coached_at: null,
      sample_run_conversation_id: null,
      setPhase: (phase) => set({ onboarding_phase: phase }),
      markItRequestSent: () => set({ it_request_sent_at: new Date().toISOString() }),
      markSampleWorked: () => set({ sample_worked_at: new Date().toISOString() }),
      markMyFileWorked: () => set({ my_file_worked_at: new Date().toISOString() }),
      markPermissionCoached: () => set({ permission_coached_at: new Date().toISOString() }),
      dismissExample: () => set({ example_dismissed: true }),
      setSampleRunConversationId: (id) => set({ sample_run_conversation_id: id }),
      completeOnboarding: () => set({ onboarding_phase: "complete" }),
    }),
    {
      name: "tamtri-onboarding-state",
      version: ONBOARDING_VERSION,
      storage: createJSONStorage(() => persistedStorage),
      partialize: (state) => ({
        onboarding_version: state.onboarding_version,
        onboarding_phase: state.onboarding_phase,
        it_request_sent_at: state.it_request_sent_at,
        sample_worked_at: state.sample_worked_at,
        my_file_worked_at: state.my_file_worked_at,
        example_dismissed: state.example_dismissed,
        permission_coached_at: state.permission_coached_at,
      }),
    },
  ),
);

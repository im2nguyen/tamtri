import { useEffect, useMemo, useState } from "react";
import { usePathname } from "expo-router";

import { useConversationList } from "@/hooks/use-conversations";
import { useReadiness } from "@/hooks/use-readiness";
import { normalizeRoutePath } from "@/lib/route-path";
import { useOnboardingStore, type OnboardingPhase } from "@/stores/onboarding-store";

const ONBOARDING_PREFIX = "/onboarding";

function phaseRoute(phase: OnboardingPhase): string | null {
  switch (phase) {
    case "welcome":
      return "/onboarding";
    case "gate":
      return "/onboarding/gate";
    case "starter":
      return "/onboarding/starter";
    case "complete":
      return null;
  }
}

/** Map gate/starter to the readiness-correct step without mutating persisted phase in an effect. */
function resolveOnboardingPhase(phase: OnboardingPhase, readyCount: number): OnboardingPhase {
  if (phase === "gate" && readyCount > 0) return "starter";
  if (phase === "starter" && readyCount === 0) return "gate";
  return phase;
}

function onboardingTargetRoute(phase: OnboardingPhase, readyCount: number): string | null {
  return phaseRoute(resolveOnboardingPhase(phase, readyCount));
}

export function useOnboardingGate() {
  const pathname = usePathname();
  const phase = useOnboardingStore((s) => s.onboarding_phase);
  const completeOnboarding = useOnboardingStore((s) => s.completeOnboarding);
  const { readyCount, loading: readinessLoading, refresh } = useReadiness();
  const { conversations, loading: conversationsLoading } = useConversationList();
  const [hydrated, setHydrated] = useState(() => useOnboardingStore.persist.hasHydrated());

  useEffect(() => {
    if (hydrated) return;
    return useOnboardingStore.persist.onFinishHydration(() => setHydrated(true));
  }, [hydrated]);

  const hasConversations = conversations.length > 0;
  const onOnboardingRoute =
    pathname === ONBOARDING_PREFIX || pathname.startsWith(`${ONBOARDING_PREFIX}/`);

  // Returning users with existing threads skip the one-time wizard.
  useEffect(() => {
    if (!hydrated || conversationsLoading || !hasConversations || phase === "complete") {
      return;
    }
    if (onOnboardingRoute) {
      return;
    }
    completeOnboarding();
  }, [
    completeOnboarding,
    conversationsLoading,
    hasConversations,
    hydrated,
    onOnboardingRoute,
    phase,
  ]);

  const shouldShowOnboarding = phase !== "complete" && !hasConversations;

  const targetRoute = useMemo(() => {
    if (!shouldShowOnboarding) return null;
    return onboardingTargetRoute(phase, readyCount);
  }, [phase, readyCount, shouldShowOnboarding]);

  const shouldRedirect =
    hydrated &&
    !readinessLoading &&
    !conversationsLoading &&
    shouldShowOnboarding &&
    targetRoute !== null &&
    normalizeRoutePath(pathname) !== normalizeRoutePath(targetRoute) &&
    !pathname.startsWith("/settings") &&
    !pathname.startsWith("/conversation");

  const shouldLeaveOnboarding = hydrated && phase === "complete" && onOnboardingRoute;

  return {
    hydrated,
    phase,
    readyCount,
    hasConversations,
    shouldShowOnboarding,
    targetRoute,
    onOnboardingRoute,
    shouldRedirect,
    shouldLeaveOnboarding,
    refreshReadiness: refresh,
  };
}

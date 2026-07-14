import { usePathname, useRouter } from "expo-router";
import { useEffect, useRef, type ReactNode } from "react";

import { useOnboardingGate } from "@/hooks/use-onboarding-gate";
import { normalizeRoutePath } from "@/lib/route-path";

interface OnboardingRouterProps {
  children: ReactNode;
}

/** Redirects between main app and onboarding based on persisted phase and agent readiness. */
export function OnboardingRouter({ children }: OnboardingRouterProps) {
  const pathname = usePathname();
  const router = useRouter();
  const { hydrated, shouldRedirect, shouldLeaveOnboarding, targetRoute } = useOnboardingGate();
  const lastReplaceRef = useRef<string | null>(null);

  useEffect(() => {
    if (!hydrated) return;

    const current = normalizeRoutePath(pathname);
    if (targetRoute && current === normalizeRoutePath(targetRoute)) {
      lastReplaceRef.current = null;
    }
  }, [hydrated, pathname, targetRoute]);

  useEffect(() => {
    if (!hydrated) return;

    if (shouldLeaveOnboarding && pathname.startsWith("/onboarding")) {
      const dest = "/";
      if (lastReplaceRef.current !== dest) {
        lastReplaceRef.current = dest;
        router.replace(dest);
      }
      return;
    }

    if (shouldRedirect && targetRoute) {
      const dest = normalizeRoutePath(targetRoute);
      if (lastReplaceRef.current !== dest) {
        lastReplaceRef.current = dest;
        router.replace(targetRoute);
      }
    }
  }, [hydrated, pathname, router, shouldLeaveOnboarding, shouldRedirect, targetRoute]);

  return children;
}

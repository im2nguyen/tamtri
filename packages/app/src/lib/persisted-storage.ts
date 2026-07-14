import AsyncStorage from "@react-native-async-storage/async-storage";
import { Platform } from "react-native";
import type { StateStorage } from "zustand/middleware";

/** Web uses localStorage; native uses AsyncStorage. */
export const persistedStorage: StateStorage = {
  getItem: (name) => {
    if (Platform.OS === "web") {
      if (typeof localStorage === "undefined") {
        return Promise.resolve(null);
      }
      return Promise.resolve(localStorage.getItem(name));
    }
    return AsyncStorage.getItem(name);
  },
  setItem: (name, value) => {
    if (Platform.OS === "web") {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(name, value);
      }
      return Promise.resolve();
    }
    return AsyncStorage.setItem(name, value);
  },
  removeItem: (name) => {
    if (Platform.OS === "web") {
      if (typeof localStorage !== "undefined") {
        localStorage.removeItem(name);
      }
      return Promise.resolve();
    }
    return AsyncStorage.removeItem(name);
  },
};

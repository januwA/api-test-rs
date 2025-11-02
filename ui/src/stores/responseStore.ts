import { create } from "zustand";
import { HttpResponse } from "../types";

interface ResponseState {
  currentResponse: HttpResponse | null;
  setResponse: (response: HttpResponse | null) => void;
}

export const useResponseStore = create<ResponseState>((set) => ({
  currentResponse: null,
  setResponse: (response) => set({ currentResponse: response }),
}));

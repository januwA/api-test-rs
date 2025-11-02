import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { message } from "antd";
import { Project, HttpTest, PairUi } from "../types";

interface ProjectState {
  project: Project | null;
  projectPath: string | null; // 当前项目文件路径
  selectedTest: { groupIndex: number; testIndex: number } | null;

  // Actions
  setProject: (project: Project, path?: string | null, resetSelection?: boolean) => void;
  selectTest: (groupIndex: number, testIndex: number) => void;
  updateProjectName: (name: string) => void;
  saveProject: () => Promise<void>; // 静默保存到当前路径
  saveProjectAs: (path: string) => Promise<void>; // 另存为到指定路径

  // Variables
  updateVariable: (key: string, value: string) => void;
  addVariable: (variable: PairUi) => void;
  deleteVariable: (key: string) => void;

  // Groups
  addGroup: (name: string) => void;
  deleteGroup: (index: number) => void;
  copyGroup: (index: number) => void;
  updateGroup: (index: number, name: string) => void;
  updateGroupName: (groupIndex: number, name: string) => void;

  // Tests
  addTest: (groupIndex: number, test: HttpTest) => void;
  deleteTest: (groupIndex: number, testIndex: number) => void;
  updateTest: (groupIndex: number, testIndex: number, test: HttpTest) => void;
  updateTestName: (groupIndex: number, testIndex: number, name: string) => void;
  copyTest: (groupIndex: number, testIndex: number) => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  projectPath: null,
  selectedTest: null,

  setProject: (project, path = null, resetSelection = false) =>
    set((state) => ({
      project,
      projectPath: path ?? state.projectPath,
      selectedTest: resetSelection ? null : state.selectedTest,
    })),

  selectTest: (groupIndex, testIndex) =>
    set({ selectedTest: { groupIndex, testIndex } }),

  // 静默保存到当前路径
  saveProject: async () => {
    const { project, projectPath } = get();

    if (!project) {
      message.warning("没有可保存的项目");
      throw new Error("No project to save");
    }

    if (!projectPath) {
      message.warning("项目尚未保存过，请使用另存为");
      throw new Error("No project path set");
    }

    try {
      // 从路径中提取目录
      const parts = projectPath.split(/[/\\]/);
      parts.pop(); // 移除文件名
      const dir = parts.join("/") || ".";

      await invoke("save_project", {
        dir,
        project,
        fontSize: 14,
      });

      message.success("项目已保存", 1);
    } catch (error) {
      console.error("保存项目失败:", error);
      message.error(`保存失败: ${error}`);
      throw error;
    }
  },

  // 另存为到指定路径
  saveProjectAs: async (path: string) => {
    const { project } = get();

    if (!project) {
      message.warning("没有可保存的项目");
      throw new Error("No project to save");
    }

    try {
      // 从路径中提取目录
      const parts = path.split(/[/\\]/);
      parts.pop(); // 移除文件名
      const dir = parts.join("/") || ".";

      await invoke("save_project", {
        dir,
        project,
        fontSize: 14,
      });

      // 更新项目路径
      set({ projectPath: path });

      message.success("项目已保存", 1);
    } catch (error) {
      console.error("保存项目失败:", error);
      message.error(`保存失败: ${error}`);
      throw error;
    }
  },

  updateProjectName: (name) =>
    set((state) => {
      if (!state.project) return state;
      return {
        project: {
          ...state.project,
          name,
        },
      };
    }),

  updateVariable: (key, value) =>
    set((state) => {
      if (!state.project) return state;

      const variables = state.project.variables.map((v) =>
        v.key === key ? { ...v, value } : v
      );

      return {
        project: {
          ...state.project,
          variables,
        },
      };
    }),

  addVariable: (variable) =>
    set((state) => {
      if (!state.project) return state;

      return {
        project: {
          ...state.project,
          variables: [...state.project.variables, variable],
        },
      };
    }),

  deleteVariable: (key) =>
    set((state) => {
      if (!state.project) return state;

      return {
        project: {
          ...state.project,
          variables: state.project.variables.filter((v) => v.key !== key),
        },
      };
    }),

  addGroup: (name) =>
    set((state) => {
      if (!state.project) return state;

      return {
        project: {
          ...state.project,
          groups: [
            ...state.project.groups,
            { name, children: [] },
          ],
        },
      };
    }),

  deleteGroup: (index) =>
    set((state) => {
      if (!state.project) return state;

      return {
        project: {
          ...state.project,
          groups: state.project.groups.filter((_, i) => i !== index),
        },
      };
    }),

  copyGroup: (index) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      const group = { ...groups[index] };
      group.name = `${group.name} - Copy`;
      // Deep copy the children (tests)
      group.children = group.children.map(test => ({ ...test }));

      return {
        project: {
          ...state.project,
          groups: [
            ...groups.slice(0, index + 1),
            group,
            ...groups.slice(index + 1),
          ],
        },
      };
    }),

  updateGroup: (index, name) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      groups[index] = { ...groups[index], name };

      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  updateGroupName: (groupIndex, name) =>
    set((state) => {
      if (!state.project) return state;
      const groups = [...state.project.groups];
      if (groups[groupIndex]) {
        groups[groupIndex] = { ...groups[groupIndex], name };
      }
      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  addTest: (groupIndex, test) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      groups[groupIndex] = {
        ...groups[groupIndex],
        children: [...groups[groupIndex].children, test],
      };

      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  deleteTest: (groupIndex, testIndex) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      groups[groupIndex] = {
        ...groups[groupIndex],
        children: groups[groupIndex].children.filter((_, i) => i !== testIndex),
      };

      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  updateTest: (groupIndex, testIndex, test) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      groups[groupIndex].children[testIndex] = test;

      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  updateTestName: (groupIndex, testIndex, name) =>
    set((state) => {
      if (!state.project) return state;
      const groups = [...state.project.groups];
      if (groups[groupIndex] && groups[groupIndex].children[testIndex]) {
        groups[groupIndex].children[testIndex] = {
          ...groups[groupIndex].children[testIndex],
          name,
        };
      }
      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),

  copyTest: (groupIndex, testIndex) =>
    set((state) => {
      if (!state.project) return state;

      const groups = [...state.project.groups];
      const test = { ...groups[groupIndex].children[testIndex] };
      test.name = `${test.name} - Copy`;

      groups[groupIndex] = {
        ...groups[groupIndex],
        children: [
          ...groups[groupIndex].children.slice(0, testIndex + 1),
          test,
          ...groups[groupIndex].children.slice(testIndex + 1),
        ],
      };

      return {
        project: {
          ...state.project,
          groups,
        },
      };
    }),
}));

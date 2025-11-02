// 数据类型定义

export interface PairUi {
  key: string;
  value: string;
  disable: boolean;
}

export enum Method {
  OPTIONS = "OPTIONS",
  GET = "GET",
  POST = "POST",
  PUT = "PUT",
  DELETE = "DELETE",
  HEAD = "HEAD",
  TRACE = "TRACE",
  CONNECT = "CONNECT",
  PATCH = "PATCH",
  WS = "WS",
}

export enum RequestTab {
  Params = "Params",
  Headers = "Headers",
  Body = "Body",
  Scripts = "Scripts",
  Curl = "Curl",
}

export enum RequestBodyTab {
  Raw = "Raw",
  Form = "Form",
  FormData = "FormData",
}

export enum RequestBodyRawType {
  Json = "Json",
  Text = "Text",
  Form = "Form",
  XML = "XML",
  BinaryFile = "BinaryFile",
}

export enum ResponseTab {
  Data = "Data",
  Header = "Header",
  Stats = "Stats",
}

export interface HttpRequestConfig {
  method: Method;
  url: string;
  body_tab_ui: RequestBodyTab;
  query: PairUi[];
  header: PairUi[];
  body_form: PairUi[];
  body_form_data: PairUi[];
  body_raw: string;
  body_raw_type: RequestBodyRawType;
  pre_request_script: string;
  post_response_script: string;
  script_enabled: boolean;
}

export interface HttpResponse {
  status: number;
  version: string;
  headers_str: string;
  request_headers_str: string;
  data_vec?: number[];
  duration: number;
  request_size: number;
  response_size: number;
  modified_vars?: PairUi[];
  // 批量请求相关字段
  isBatch?: boolean;
  total?: number;
  success?: number;
  failed?: number;
  responses?: HttpResponse[];
  avgDuration?: number;
  minDuration?: number;
  maxDuration?: number;
}

export interface HttpTest {
  name: string;
  tab_ui: RequestTab;
  send_count_ui: string;
  request: HttpRequestConfig;
}

export interface Group {
  name: string;
  children: HttpTest[];
}

export interface Project {
  name: string;
  groups: Group[];
  variables: PairUi[];
}

export interface ProjectWithPath {
  project: Project;
  path: string;
}

export interface AppConfig {
  project_path: string;
  font_size: number;
}

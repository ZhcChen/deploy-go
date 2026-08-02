import { UsersApi } from "../../api/generated/apis/UsersApi";
import { apiConfiguration } from "../../api/http-client";

export const usersApi = new UsersApi(apiConfiguration);

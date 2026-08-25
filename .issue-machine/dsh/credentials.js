import { readFile } from "node:fs/promises";
import z from "@deepseek-ai/schemastery";
import { CredentialProvider } from "@deepseek-ai/dsh-credentials";

export default class OpenCodeCredentialProvider extends CredentialProvider {
	static Config = z.object({
		openCodeAuthPath: z.string().required(),
		openCodeRef: z.string().required(),
		openCodeValue: z.string().required(),
		openRouterRef: z.string().required(),
		openRouterProvider: z.string().required(),
	});

	constructor(ctx, config) {
		super(ctx);
		this.config = config;
	}

	async openRouter() {
		const document = JSON.parse(await readFile(this.config.openCodeAuthPath, "utf8"));
		const credential = document[this.config.openRouterProvider];
		if (credential?.type !== "api" || typeof credential.key !== "string" || credential.key.trim() === "") {
			throw new Error(`OpenCode has no API credential for provider ${JSON.stringify(this.config.openRouterProvider)}`);
		}
		return credential.key;
	}

	async resolve(ref) {
		if (ref === this.config.openCodeRef) {
			return { value: this.config.openCodeValue, source: "recipe-machine" };
		}
		if (ref === this.config.openRouterRef) {
			return { value: await this.openRouter(), source: "opencode-auth" };
		}
		return undefined;
	}

	async describe(ref) {
		if (ref === this.config.openCodeRef) {
			return { configured: true, source: "recipe-machine", writable: false };
		}
		if (ref === this.config.openRouterRef) {
			await this.openRouter();
			return { configured: true, source: "opencode-auth", writable: false };
		}
		return { configured: false, writable: false };
	}

	async set() {
		throw new Error("Recipe Machine DSH credentials are read-only");
	}

	async unset() {
		throw new Error("Recipe Machine DSH credentials are read-only");
	}

	async readRecord() {
		return undefined;
	}

	async describeRecord() {
		return { configured: false, writable: false };
	}

	async listRecords() {
		return [];
	}

	async modifyRecord() {
		throw new Error("Recipe Machine DSH credentials are read-only");
	}

	async deleteRecord() {
		throw new Error("Recipe Machine DSH credentials are read-only");
	}
}

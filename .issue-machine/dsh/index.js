import { randomUUID } from "node:crypto";
import { chmod, rm } from "node:fs/promises";
import { createServer } from "node:net";
import z from "@deepseek-ai/schemastery";
import { installModelSelection } from "@deepseek-ai/dsh-agent";
import { createUserMessage } from "@deepseek-ai/dsh-llm";
import { SessionId } from "@deepseek-ai/dsh-session";

export const name = "recipe-machine-dsh";
export const inject = ["agents", "tools"];
export const Config = z.object({
	socketPath: z.string().required(),
	repository: z.string().required(),
	openCodeProvider: z.string().required(),
	maxPromptBytes: z.number().min(1).required(),
	maxTurns: z.number().min(1).required(),
});

function summarize(events, firstSeq) {
	let text = "";
	let reason;
	for (const event of events) {
		if (event.seq < firstSeq) continue;
		if (event.type === "assistant/message") {
			const current = event.data.message.content
				.filter((block) => block.type === "text")
				.map((block) => block.text)
				.join("");
			if (current !== "") text = current;
		}
		if (event.type === "turn/end") reason = event.data.reason;
	}
	return { text, reason };
}

function errorText(value) {
	return value instanceof Error ? value.message : String(value);
}

function response(socket, status, text) {
	const body = Buffer.from(text, "utf8");
	socket.write(`RESULT ${status} ${body.length}\n`);
	socket.write(body);
}

function outcomeResponse(socket, outcome) {
	if (outcome.reason?.kind === "error") {
		response(socket, "ERROR", `${outcome.reason.error.code}: ${outcome.reason.error.message}`);
		return;
	}
	if (outcome.reason?.kind === "aborted") {
		response(socket, "ABORTED", "");
		return;
	}
	if (outcome.text === "") {
		response(socket, "ERROR", `DSH turn ended without assistant text (${outcome.reason?.kind ?? "unknown"})`);
		return;
	}
	response(socket, "COMPLETED", outcome.text);
}

class Connection {
	constructor(ctx, config, socket) {
		this.ctx = ctx;
		this.config = config;
		this.socket = socket;
		this.buffer = Buffer.alloc(0);
		this.promptBytes = undefined;
		this.promptRoute = undefined;
		this.handle = undefined;
		this.active = undefined;
		this.turns = 0;
		this.closed = false;
		socket.on("data", (data) => this.receive(data));
		socket.on("error", () => this.close());
		socket.on("close", () => this.close());
	}

	receive(data) {
		if (this.closed) return;
		this.buffer = Buffer.concat([this.buffer, data]);
		try {
			this.parse();
		} catch (error) {
			response(this.socket, "ERROR", errorText(error));
			this.socket.destroy();
		}
	}

	parse() {
		while (!this.closed) {
			if (this.promptBytes !== undefined) {
				if (this.buffer.length < this.promptBytes) return;
				const prompt = this.buffer.subarray(0, this.promptBytes).toString("utf8");
				this.buffer = this.buffer.subarray(this.promptBytes);
				const route = this.promptRoute;
				this.promptBytes = undefined;
				this.promptRoute = undefined;
				this.startPrompt(route.provider, route.model, prompt);
				continue;
			}
			const newline = this.buffer.indexOf(0x0a);
			if (newline < 0) return;
			const line = this.buffer.subarray(0, newline).toString("utf8");
			this.buffer = this.buffer.subarray(newline + 1);
			if (line === "CANCEL") {
				this.handle?.agent.cancel({ kind: "user" });
				continue;
			}
			const match = /^PROMPT ([^ ]+) ([^ ]+) ([0-9]+)$/u.exec(line);
			if (match === null) throw new Error("invalid Recipe Machine DSH command");
			const bytes = Number(match[3]);
			if (!Number.isSafeInteger(bytes) || bytes > this.config.maxPromptBytes) {
				throw new Error(`Recipe Machine DSH prompt exceeds ${this.config.maxPromptBytes} bytes`);
			}
			if (this.active !== undefined) throw new Error("a Recipe Machine DSH turn is already active on this connection");
			if (this.turns >= this.config.maxTurns) throw new Error(`Recipe Machine DSH connection permits ${this.config.maxTurns} turns`);
			this.promptRoute = { provider: match[1], model: match[2] };
			this.promptBytes = bytes;
		}
	}

	startPrompt(provider, model, prompt) {
		this.active = this.runPrompt(provider, model, prompt)
			.catch((error) => {
				if (!this.closed) response(this.socket, "ERROR", errorText(error));
			})
			.finally(() => {
				this.active = undefined;
			});
	}

	async agent(provider, model) {
		if (provider !== "opencode" && provider !== "openrouter") {
			throw new Error(`unsupported Recipe Machine DSH provider ${JSON.stringify(provider)}`);
		}
		const route = provider === "opencode" ? this.config.openCodeProvider : provider;
		if (this.handle !== undefined) {
			if (this.handle.agent.options.provider !== route || this.handle.agent.options.model !== model) {
				throw new Error("a Recipe Machine DSH connection cannot change provider or model");
			}
			return this.handle.agent;
		}
		const selection = { provider: route, model };
		this.handle = await this.ctx.agents.create({
			sessionId: SessionId(`recipe-machine-${randomUUID()}`),
			meta: { cwd: this.config.repository },
			agentOptions: selection,
			setup: (agentCtx) => {
				installModelSelection(agentCtx, { current: selection, assembled: undefined });
				agentCtx.tools.restrict({
					allow: [
						"read",
						"glob",
						"grep",
						"mcp__recipe_issues__search_issues",
						"mcp__recipe_issues__read_issue",
					],
				});
			},
		});
		await this.handle.agent.whenIdle();
		return this.handle.agent;
	}

	async runPrompt(provider, model, prompt) {
		const agent = await this.agent(provider, model);
		const firstSeq = agent.session.seq;
		this.turns += 1;
		agent.followup(createUserMessage({
			content: [{ type: "text", text: prompt }],
			source: { kind: "user" },
		}));
		await agent.whenIdle();
		if (!this.closed) outcomeResponse(this.socket, summarize(agent.session.events, firstSeq));
	}

	async close() {
		if (this.closed) return;
		this.closed = true;
		this.handle?.agent.cancel({ kind: "disposed" });
		await this.active?.catch(() => {});
		await this.handle?.dispose();
	}
}

async function start(ctx, config) {
	await ctx.get("loader")?.await();
	await rm(config.socketPath, { force: true });
	const connections = new Set();
	const server = createServer((socket) => {
		const connection = new Connection(ctx, config, socket);
		connections.add(connection);
		socket.once("close", () => connections.delete(connection));
	});
	await new Promise((resolve, reject) => {
		server.once("error", reject);
		server.listen(config.socketPath, () => {
			server.off("error", reject);
			resolve();
		});
	});
	await chmod(config.socketPath, 0o600);
	ctx.effect(() => async () => {
		const closed = new Promise((resolve) => server.close(resolve));
		for (const connection of connections) connection.socket.destroy();
		await Promise.all([closed, ...[...connections].map((connection) => connection.close())]);
		await rm(config.socketPath, { force: true });
	}, "recipeMachineDsh.listen");
	process.stdout.write(`recipe-machine-dsh: listening on ${config.socketPath}\n`);
}

export function apply(ctx, config) {
	start(ctx, config).catch((error) => {
		process.stderr.write(`recipe-machine-dsh: ${errorText(error)}\n`);
		process.exitCode = 1;
		ctx.get("appExit")?.(1);
	});
}

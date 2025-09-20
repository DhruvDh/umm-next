import { Application, Router } from "oak";
import {
  ChatCompletionRequestMessage,
  CreateChatCompletionRequest,
  OpenAI,
} from "openai";
import { createClient } from "@supabase/supabase-js";

const router = new Router();
const app = new Application();
const openai = new OpenAI();

app.use((ctx, next) => {
  ctx.response.headers.set("Access-Control-Allow-Origin", "*");
  ctx.response.headers.set("Access-Control-Allow-Methods", "OPTIONS,GET");
  ctx.response.headers.set(
    "Access-Control-Allow-Headers",
    "Content-Type,Authorization"
  );
  return next();
});

router
  .options("/", (ctx) => {
    ctx.response.status = 200;
    ctx.response.statusText = "OK";
  })
  .options("/:prompt_id", (ctx) => {
    ctx.response.status = 200;
    ctx.response.statusText = "OK";
  })
  .get("/:prompt_id", async (ctx) => {
    const promptID = ctx.params.prompt_id;
    const target = ctx.sendEvents();

    const supabaseClient = createClient(
      Deno.env.get("SUPABASE_URL") ?? "",
      Deno.env.get("SUPABASE_ANON_KEY") ?? ""
    );

    const { data, error } = await supabaseClient
      .from("prompts")
      .select("*")
      .eq("id", promptID)
      .select("messages")
      .single();

    if (error) throw error;
    let messages: Array<ChatCompletionRequestMessage>;
    try {
      messages =
        (data.messages as unknown as Array<ChatCompletionRequestMessage>) ?? [
          {
            role: "system",
            content:
              "- You are an AI teaching assistant at UNC, Charlotte.\n- The course uses a computer program is used to generate a prompt, then the prompt is shared with you.\n- This prompt is helpful explanation of errors or assignment grading feedback.\n- The explanation/feedback is what the user - a Student - is here for.\n- However, something has gone wrong with the program that was to generate this prompt and now you cannot help the student. Please explain this to the student.\n- **Note: The student cannot respond, so do not expect him to**.\n> Respond in markdown syntax only.",
          },
        ];
    } catch (_e) {
      messages = [
        {
          role: "system",
          content:
            "- You are an AI teaching assistant at UNC, Charlotte.\n- The course uses a computer program is used to generate a prompt, then the prompt is shared with you.\n- This prompt is helpful explanation of errors or assignment grading feedback.\n- The explanation/feedback is what the user - a Student - is here for.\n- However, something has gone wrong with the program that was to generate this prompt and now you cannot help the student. Please explain this to the student.\n- **Note: The student cannot respond, so do not expect him to**.\n> Respond in markdown syntax only.",
        },
      ];
    }

    const completionConfig: CreateChatCompletionRequest = {
      model: "gpt-3.5-turbo",
      temperature: 0,
      top_p: 1,
      n: 1,
      frequency_penalty: 0.0,
      presence_penalty: 0.0,
      stream: true,
      messages,
    };

    const resp = await openai.chat.completions
      .create(completionConfig)
      .asResponse();
    if (resp.status !== 200) {
      throw new Error(await resp.text());
    }

    resp.body
      ?.pipeThrough(new TextDecoderStream())
      .pipeThrough(
        new TransformStream({
          transform(chunk, controller) {
            chunk
              .split("data: ")
              .slice(1)
              .forEach((i) => {
                try {
                  controller.enqueue(JSON.parse(i));
                } catch (_e) {
                  controller.enqueue(i);
                }
              });
          },
        })
      )
      .pipeTo(
        new WritableStream({
          write(chunk) {
            target.dispatchMessage(chunk);
          },
        })
      );
  });

app.use(router.routes());
await app.listen({ port: 8000 });

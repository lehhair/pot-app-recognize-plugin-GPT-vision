# Pot-App 文字识别插件 GPT-vison

通过调用GPT-vision模型对图片进行识别

你可以这样填写参数

API KEY: sk-xxxxx

API Endpoint: https://api.openai.com/v1/chat/completions

Model: gpt-4o

Prompt: Please analyze the provided image, recognize and extract all the text content, and describe the main content of the image. Your response will be used as input for the next AI assistant or translation assistant. Please return the result in `Chinese (cn)`.

这里输出语言可以自行选择，也可以保持原文，看你喜好

Stream: False (首先这里true没有应用场景，其次会报错，仅作预留)

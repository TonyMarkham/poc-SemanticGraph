using System.Text.Json;
using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record JsonRpcErrorDto(
    [property: JsonPropertyName("code")] long Code,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("data")] JsonElement? Data);

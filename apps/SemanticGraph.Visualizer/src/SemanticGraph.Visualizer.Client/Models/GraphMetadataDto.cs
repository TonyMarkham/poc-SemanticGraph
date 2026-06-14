using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphMetadataDto(
    [property: JsonPropertyName("databasePath")] string DatabasePath,
    [property: JsonPropertyName("limit")] int Limit,
    [property: JsonPropertyName("nodeCount")] int NodeCount,
    [property: JsonPropertyName("edgeCount")] int EdgeCount);

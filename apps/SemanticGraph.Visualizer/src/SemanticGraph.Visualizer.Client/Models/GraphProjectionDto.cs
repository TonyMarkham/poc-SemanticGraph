using System.Text.Json.Serialization;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphProjectionDto(
    [property: JsonPropertyName("nodes")] IReadOnlyList<GraphNodeDto> Nodes,
    [property: JsonPropertyName("edges")] IReadOnlyList<GraphEdgeDto> Edges,
    [property: JsonPropertyName("metadata")] GraphMetadataDto Metadata);

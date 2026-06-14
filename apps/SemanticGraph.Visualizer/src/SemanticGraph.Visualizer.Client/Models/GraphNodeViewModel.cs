namespace SemanticGraph.Visualizer.Client.Models;

public sealed record GraphNodeViewModel(
    string Id,
    string Kind,
    string DisplayLabel,
    string? QualifiedName,
    string Language,
    string? SourceFilePath)
{
    public bool IsFile => string.Equals(Kind, "file", StringComparison.OrdinalIgnoreCase);

    public string Tooltip => QualifiedName ?? SourceFilePath ?? DisplayLabel;

    public string KindCssClass => Kind.Replace('_', '-').ToLowerInvariant();

    public static GraphNodeViewModel FromDto(GraphNodeDto dto)
    {
        return new GraphNodeViewModel(
            dto.Id,
            dto.Kind,
            dto.DisplayLabel,
            dto.QualifiedName,
            dto.Language,
            dto.SourceFilePath);
    }
}

//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_template_file_response.g.dart';

/// ApplicationTemplateFileResponse
///
/// Properties:
/// * [content]
/// * [delivery]
/// * [deployPath]
/// * [description]
/// * [digest]
/// * [editable]
/// * [format]
/// * [label]
/// * [language]
/// * [path]
/// * [recommendedChanges]
/// * [role]
/// * [sensitive]
@BuiltValue()
abstract class ApplicationTemplateFileResponse implements Built<ApplicationTemplateFileResponse, ApplicationTemplateFileResponseBuilder> {
  @BuiltValueField(wireName: r'content')
  String? get content;

  @BuiltValueField(wireName: r'delivery')
  String get delivery;

  @BuiltValueField(wireName: r'deploy_path')
  String? get deployPath;

  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'digest')
  String get digest;

  @BuiltValueField(wireName: r'editable')
  bool get editable;

  @BuiltValueField(wireName: r'format')
  String get format;

  @BuiltValueField(wireName: r'label')
  String get label;

  @BuiltValueField(wireName: r'language')
  String get language;

  @BuiltValueField(wireName: r'path')
  String get path;

  @BuiltValueField(wireName: r'recommended_changes')
  String get recommendedChanges;

  @BuiltValueField(wireName: r'role')
  String get role;

  @BuiltValueField(wireName: r'sensitive')
  bool get sensitive;

  ApplicationTemplateFileResponse._();

  factory ApplicationTemplateFileResponse([void updates(ApplicationTemplateFileResponseBuilder b)]) = _$ApplicationTemplateFileResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationTemplateFileResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationTemplateFileResponse> get serializer => _$ApplicationTemplateFileResponseSerializer();
}

class _$ApplicationTemplateFileResponseSerializer implements PrimitiveSerializer<ApplicationTemplateFileResponse> {
  @override
  final Iterable<Type> types = const [ApplicationTemplateFileResponse, _$ApplicationTemplateFileResponse];

  @override
  final String wireName = r'ApplicationTemplateFileResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationTemplateFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.content != null) {
      yield r'content';
      yield serializers.serialize(
        object.content,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'delivery';
    yield serializers.serialize(
      object.delivery,
      specifiedType: const FullType(String),
    );
    if (object.deployPath != null) {
      yield r'deploy_path';
      yield serializers.serialize(
        object.deployPath,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'digest';
    yield serializers.serialize(
      object.digest,
      specifiedType: const FullType(String),
    );
    yield r'editable';
    yield serializers.serialize(
      object.editable,
      specifiedType: const FullType(bool),
    );
    yield r'format';
    yield serializers.serialize(
      object.format,
      specifiedType: const FullType(String),
    );
    yield r'label';
    yield serializers.serialize(
      object.label,
      specifiedType: const FullType(String),
    );
    yield r'language';
    yield serializers.serialize(
      object.language,
      specifiedType: const FullType(String),
    );
    yield r'path';
    yield serializers.serialize(
      object.path,
      specifiedType: const FullType(String),
    );
    yield r'recommended_changes';
    yield serializers.serialize(
      object.recommendedChanges,
      specifiedType: const FullType(String),
    );
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    yield r'sensitive';
    yield serializers.serialize(
      object.sensitive,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationTemplateFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationTemplateFileResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.content = valueDes;
          break;
        case r'delivery':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.delivery = valueDes;
          break;
        case r'deploy_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.deployPath = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.digest = valueDes;
          break;
        case r'editable':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.editable = valueDes;
          break;
        case r'format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.format = valueDes;
          break;
        case r'label':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.label = valueDes;
          break;
        case r'language':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.language = valueDes;
          break;
        case r'path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.path = valueDes;
          break;
        case r'recommended_changes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.recommendedChanges = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'sensitive':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.sensitive = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationTemplateFileResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationTemplateFileResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
